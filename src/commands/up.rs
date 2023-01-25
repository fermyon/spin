use std::{fmt::Debug, path::{Path, PathBuf}};

use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, Subcommand, ArgGroup, ArgAction, Arg, FromArgMatches};
use reqwest::Url;
use spin_manifest::{Application, ApplicationTrigger};
use spin_trigger::cli::{SPIN_LOCKED_URL, SPIN_WORKING_DIR};

use crate::{args::{component::ComponentOptions, app_source::AppSource}, dispatch::{Dispatch, Action}};
use async_trait::async_trait;
use crate::dispatch::Runner;


impl UpCommand {
    pub async fn load(&self) -> Result<Application> {
        let Self { source, components, trigger } = self;
        let mut app = source.load(&components.working_dir()?).await?;
        if !components.env.is_empty() {
            for c in app.components.iter_mut() {
                c.wasm.environment.extend(components.env.iter().cloned());
            }
        }
        Ok(app)
    }
}

/// Start the Fermyon runtime.
#[derive(Parser, Debug, Clone, Default)]
#[command(about = "Start the Spin application")]
#[command(allow_hyphen_values = true)]
pub struct UpCommand {
    /// The app location to start ()
    #[command(flatten, next_help_heading = "App Source")]
    #[group(skip)]
    pub source: AppSource,
    // Options to pass through to the app's components
    #[command(flatten, next_help_heading = "Component Options")]
    pub components: ComponentOptions,
    // Options to pass through to the trigger
    #[arg(hide = true, trailing_var_arg = true, help_heading = "Trigger Options")]
    pub trigger: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Flag<T>(Option<T>);

impl<T> Flag<T> {
    fn get(&self) -> Result<&T> {
        self.0.as_ref().ok_or(anyhow!("flag disabled"))
    }
}

pub trait FlagShortcut: FromArgMatches + Parser + Clone + Send + Sync + Sized + Debug {
    const GROUP: &'static str;
    const LONG: &'static str;
    const SHORT: char;
    const ACTION: ArgAction;
}

impl<T> FromArgMatches for Flag<T> where T: FlagShortcut {
    fn from_arg_matches(matches: &clap::ArgMatches) -> std::result::Result<Self, clap::Error> {
        match matches.try_get_one::<bool>(T::LONG) {
            Ok(flag) => if let Some(flag) = flag {
                match T::from_arg_matches(matches) {
                Ok(inner) if *flag => Ok(Self(Some(inner))),
                Ok(inner) => unreachable!("{flag} {inner:?}"),
                Err(error) => Err(error),
            }
            } else { Ok(Self(None)) },
            Err(e) => unreachable!("{e}"),
        }
    }

    fn update_from_arg_matches(&mut self, matches: &clap::ArgMatches) -> std::result::Result<(), clap::Error> {
        Ok(*self = Self::from_arg_matches(matches)?)
    }
}

impl<T> Subcommand for Flag<T> where T: FlagShortcut {
  
    fn augment_subcommands(cmd: clap::Command) -> clap::Command {
        let upcmd = T::command_for_update();
        let upargs = upcmd.get_arguments();
        let newargs = upargs.map(|arg| arg.clone().group(T::GROUP));
        cmd.arg(
            Arg::new(T::LONG).long(T::LONG).short(T::SHORT).action(T::ACTION)
        ).group(ArgGroup::new(T::GROUP).requires(T::LONG)).args(newargs)
    }

    fn augment_subcommands_for_update(cmd: clap::Command) -> clap::Command {
        Self::augment_subcommands(cmd)
    }

    fn has_subcommand(_name: &str) -> bool {
        false
    }
}

impl FlagShortcut for UpCommand {
    const GROUP: &'static str = "up_args";
    const LONG: &'static str = "up";
    const SHORT: char = 'u';
    const ACTION: ArgAction = ArgAction::SetTrue;
}

#[async_trait(?Send)]
impl<T> Dispatch for Flag<T> where T: Dispatch {
    async fn dispatch(&self, action: &Action) -> Result<()> {
        self.get()?.dispatch(action).await
    }
}

#[async_trait(?Send)]
impl Dispatch for UpCommand {
    async fn run(&self) -> Result<()> {
        let app = self.load().await?;

        let trigger_type = match &app.info.trigger {
            ApplicationTrigger::Http(_) => trigger_command("http"),
            ApplicationTrigger::Redis(_) => trigger_command("redis"),
            ApplicationTrigger::External(cfg) => vec![resolve_trigger_plugin(cfg.trigger_type())?],
        };

        let working_dir = self.components.working_dir()?;

        self.run_trigger(trigger_type, app, working_dir, false)
    }
}

impl UpCommand {

    fn run_trigger(
        &self,
        trigger_type: Vec<String>,
        app: Application,
        working_dir: PathBuf,
        help: bool
    ) -> Result<(), anyhow::Error> {
        // The docs for `current_exe` warn that this may be insecure because it could be executed
        // via hard-link. I think it should be fine as long as we aren't `setuid`ing this binary.
        let mut cmd = std::process::Command::new(std::env::current_exe().unwrap());
        cmd.args(&trigger_type);
        if help { cmd.arg("--help-args-only"); }

        let locked_url = self.write_locked_app(app, &working_dir)?;
        cmd.env(SPIN_LOCKED_URL, locked_url)
            .env(SPIN_WORKING_DIR, &working_dir)
            .args(&self.trigger);
            

        tracing::trace!("Running trigger executor: {:?}", cmd);

        let mut child = cmd.spawn().context("Failed to execute trigger")?;

        // Terminate trigger executor if `spin up` itself receives a termination signal
        #[cfg(not(windows))]
        {
            // https://github.com/nix-rust/nix/issues/656
            let pid = nix::unistd::Pid::from_raw(child.id() as i32);
            ctrlc::set_handler(move || {
                if let Err(err) = nix::sys::signal::kill(pid, nix::sys::signal::SIGTERM) {
                    tracing::warn!("Failed to kill trigger handler process: {:?}", err)
                }
            })?;
        }

        let status = child.wait()?;
        if status.success() {
            Ok(())
        } else {
            bail!(status);
        }
    }

    fn write_locked_app(
        &self,
        app: spin_manifest::Application,
        working_dir: &Path,
    ) -> Result<String, anyhow::Error> {
        // Build and write app lock file
        let locked_app = spin_trigger::locked::build_locked_app(app, working_dir)?;
        let locked_path = working_dir.join("spin.lock");
        let locked_app_contents =
            serde_json::to_vec_pretty(&locked_app).context("failed to serialize locked app")?;
        std::fs::write(&locked_path, locked_app_contents)
            .with_context(|| format!("failed to write {:?}", locked_path))?;
        let locked_url = Url::from_file_path(&locked_path)
            .map_err(|_| anyhow!("cannot convert to file URL: {locked_path:?}"))?
            .to_string();

        Ok(locked_url)
    }

    // fn bindle_connection(&self) -> Option<BindleConnectionInfo> {
    //     self.server.as_ref().map(|url| {
    //         BindleConnectionInfo::new(
    //             url,
    //             self.insecure,
    //             self.bindle_username.clone(),
    //             self.bindle_password.clone(),
    //         )
    //     })
    // }
}

// enum WorkingDirectory {
//     Given(PathBuf),
//     Temporary(TempDir),
// }

// impl WorkingDirectory {
//     fn path(&self) -> &Path {
//         match self {
//             Self::Given(p) => p,
//             Self::Temporary(t) => t.path(),
//         }
//     }
// }

fn resolve_trigger_plugin(trigger_type: &str) -> Result<String> {
    use crate::commands::plugins::PluginCompatibility;
    use spin_plugins::manager::PluginManager;

    let subcommand = format!("trigger-{trigger_type}");
    let plugin_manager = PluginManager::try_default()
        .with_context(|| format!("Failed to access plugins looking for '{subcommand}'"))?;
    let plugin_store = plugin_manager.store();
    let is_installed = plugin_store
        .installed_manifests()
        .unwrap_or_default()
        .iter()
        .any(|m| m.name() == subcommand);

    if is_installed {
        return Ok(subcommand);
    }

    if let Some(known) = plugin_store
        .catalogue_manifests()
        .unwrap_or_default()
        .iter()
        .find(|m| m.name() == subcommand)
    {
        match PluginCompatibility::for_current(known) {
            PluginCompatibility::Compatible => Err(anyhow!("No built-in trigger named '{trigger_type}', but plugin '{subcommand}' is available to install")),
            _ => Err(anyhow!("No built-in trigger named '{trigger_type}', and plugin '{subcommand}' is not compatible"))
        }
    } else {
        Err(anyhow!("No built-in trigger named '{trigger_type}', and no plugin named '{subcommand}' was found"))
    }
}

fn trigger_command(trigger_type: &str) -> Vec<String> {
    vec!["trigger".to_owned(), trigger_type.to_owned()]
}
