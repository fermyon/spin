use std::{
    ffi::OsString,
    fmt::Debug,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Context, Result};
use clap::{CommandFactory, Parser};
use reqwest::Url;
use spin_app::locked::LockedApp;
use spin_loader::bindle::BindleConnectionInfo;
use spin_manifest::{Application, ApplicationTrigger};
use spin_trigger::cli::{SPIN_LOCKED_URL, SPIN_WORKING_DIR};
use tempfile::TempDir;

use crate::opts::*;

/// Start the Fermyon runtime.
#[derive(Parser, Debug, Default)]
#[clap(
    about = "Start the Spin application",
    allow_hyphen_values = true,
    disable_help_flag = true
)]
pub struct UpCommand {
    #[clap(short = 'h', long = "help")]
    pub help: bool,

    /// Path to spin.toml.
    #[clap(
        name = APP_CONFIG_FILE_OPT,
        short = 'f',
        long = "file",
        conflicts_with = BINDLE_ID_OPT,
        conflicts_with = FROM_REGISTRY_OPT,
    )]
    pub app: Option<PathBuf>,

    /// ID of application bindle.
    #[clap(
        name = BINDLE_ID_OPT,
        short = 'b',
        long = "bindle",
        conflicts_with = APP_CONFIG_FILE_OPT,
        conflicts_with = FROM_REGISTRY_OPT,
        requires = BINDLE_SERVER_URL_OPT,
    )]
    pub bindle: Option<String>,

    /// URL of bindle server.
    #[clap(
        name = BINDLE_SERVER_URL_OPT,
        long = "bindle-server",
        env = BINDLE_URL_ENV,
    )]
    pub server: Option<String>,

    /// Basic http auth username for the bindle server
    #[clap(
        name = BINDLE_USERNAME,
        long = "bindle-username",
        env = BINDLE_USERNAME,
        requires = BINDLE_PASSWORD,
    )]
    pub bindle_username: Option<String>,

    /// Basic http auth password for the bindle server
    #[clap(
        name = BINDLE_PASSWORD,
        long = "bindle-password",
        env = BINDLE_PASSWORD,
        requires = BINDLE_USERNAME,
    )]
    pub bindle_password: Option<String>,

    /// Reference to run the application from a registry.
    #[clap(
        name = FROM_REGISTRY_OPT,
        long = "from-registry",
        conflicts_with = BINDLE_ID_OPT,
        conflicts_with = APP_CONFIG_FILE_OPT,
    )]
    pub reference: Option<String>,

    /// Ignore server certificate errors from bindle server or registry
    #[clap(
        name = INSECURE_OPT,
        short = 'k',
        long = "insecure",
        takes_value = false,
    )]
    pub insecure: bool,

    /// Pass an environment variable (key=value) to all components of the application.
    #[clap(short = 'e', long = "env", parse(try_from_str = parse_env_var))]
    pub env: Vec<(String, String)>,

    /// Temporary directory for the static assets of the components.
    #[clap(long = "temp")]
    pub tmp: Option<PathBuf>,

    /// For local apps with directory mounts and no excluded files, mount them directly instead of using a temporary
    /// directory.
    ///
    /// This allows you to update the assets on the host filesystem such that the updates are visible to the guest
    /// without a restart.  This cannot be used with bindle apps or apps which use file patterns and/or exclusions.
    #[clap(long, takes_value = false, conflicts_with = BINDLE_ID_OPT)]
    pub direct_mounts: bool,

    /// All other args, to be passed through to the trigger
    #[clap(hide = true)]
    pub trigger_args: Vec<OsString>,
}

impl UpCommand {
    pub async fn run(self) -> Result<()> {
        // For displaying help, first print `spin up`'s own usage text, then
        // attempt to load an app and print trigger-type-specific usage.
        let help = self.help;
        if help {
            Self::command()
                .name("spin-up")
                .bin_name("spin up")
                .print_help()?;
            println!();
        }
        self.run_inner().await.or_else(|err| {
            if help {
                tracing::warn!("Error resolving trigger-specific help: {}", err);
                Ok(())
            } else {
                Err(err)
            }
        })
    }

    async fn run_inner(self) -> Result<()> {
        if self.help
            && self.app.is_none()
            && self.bindle.is_none()
            && self.reference.is_none()
            && !PathBuf::from(DEFAULT_MANIFEST_FILE).exists()
        {
            return self
                .run_trigger(
                    trigger_command(HELP_ARGS_ONLY_TRIGGER_TYPE),
                    TriggerExecOpts::NoApp,
                )
                .await;
        }

        let working_dir_holder = match &self.tmp {
            None => WorkingDirectory::Temporary(tempfile::tempdir()?),
            Some(d) => WorkingDirectory::Given(d.to_owned()),
        };
        let working_dir = working_dir_holder.path().canonicalize()?;

        let (trigger_cmd, exec_opts) = match (&self.app, &self.bindle, &self.reference) {
            (app, None, None) => {
                self.prepare_app_from_file(app.as_deref(), working_dir)
                    .await?
            }
            (None, Some(bindle), None) => self.prepare_app_from_bindle(bindle, working_dir).await?,
            (None, None, Some(reference)) => {
                self.prepare_app_from_oci(reference, working_dir).await?
            }
            (_, _, _) => {
                bail!("Specify only one of app file, bindle ID, or container registry reference");
            }
        };

        self.run_trigger(trigger_cmd, exec_opts).await
    }

    async fn run_trigger(
        self,
        trigger_type: Vec<String>,
        exec_opts: TriggerExecOpts,
    ) -> Result<(), anyhow::Error> {
        // The docs for `current_exe` warn that this may be insecure because it could be executed
        // via hard-link. I think it should be fine as long as we aren't `setuid`ing this binary.
        let mut cmd = std::process::Command::new(std::env::current_exe().unwrap());
        cmd.args(&trigger_type);

        match exec_opts {
            TriggerExecOpts::NoApp => {
                cmd.arg("--help-args-only");
            }
            TriggerExecOpts::Local { app, working_dir } => {
                let locked_app = spin_trigger::locked::build_locked_app(app, &working_dir)?;
                let locked_url = self.write_locked_app(&locked_app, &working_dir).await?;
                cmd.env(SPIN_LOCKED_URL, locked_url)
                    .env(SPIN_WORKING_DIR, &working_dir)
                    .args(&self.trigger_args);
            }
            TriggerExecOpts::Remote {
                locked_url,
                working_dir,
                from_registry,
            } => {
                cmd.env(SPIN_LOCKED_URL, locked_url)
                    .env(SPIN_WORKING_DIR, &working_dir)
                    .args(&self.trigger_args);
                if from_registry {
                    cmd.arg("--from-registry");
                }
            }
        }

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

    async fn write_locked_app(
        &self,
        locked_app: &LockedApp,
        working_dir: &Path,
    ) -> Result<String, anyhow::Error> {
        let locked_path = working_dir.join("spin.lock");
        let locked_app_contents =
            serde_json::to_vec_pretty(&locked_app).context("failed to serialize locked app")?;
        tokio::fs::write(&locked_path, locked_app_contents)
            .await
            .with_context(|| format!("failed to write {:?}", locked_path))?;
        let locked_url = Url::from_file_path(&locked_path)
            .map_err(|_| anyhow!("cannot convert to file URL: {locked_path:?}"))?
            .to_string();

        Ok(locked_url)
    }

    fn bindle_connection(&self) -> Option<BindleConnectionInfo> {
        self.server.as_ref().map(|url| {
            BindleConnectionInfo::new(
                url,
                self.insecure,
                self.bindle_username.clone(),
                self.bindle_password.clone(),
            )
        })
    }

    // Prepares the application for trigger execution returning the trigger command
    // to execute and the URL of the locked application.
    async fn prepare_locked_app(
        &self,
        mut locked_app: LockedApp,
        working_dir: PathBuf,
    ) -> Result<(Vec<String>, TriggerExecOpts)> {
        // Apply --env to component environments
        if !self.env.is_empty() {
            for component in locked_app.components.iter_mut() {
                component.env.extend(self.env.iter().cloned());
            }
        }

        let trigger_command = trigger_command_from_locked_app(&locked_app)?;
        let locked_url = self.write_locked_app(&locked_app, &working_dir).await?;

        let exec_opts = if self.help {
            TriggerExecOpts::NoApp
        } else {
            let from_registry = self.reference.is_some();
            TriggerExecOpts::Remote {
                locked_url,
                working_dir,
                from_registry,
            }
        };

        Ok((trigger_command, exec_opts))
    }

    async fn prepare_app_from_file(
        &self,
        path: Option<&Path>,
        working_dir: PathBuf,
    ) -> Result<(Vec<String>, TriggerExecOpts)> {
        let manifest_file = path.unwrap_or_else(|| DEFAULT_MANIFEST_FILE.as_ref());
        let bindle_connection = self.bindle_connection();

        let asset_dst = if self.direct_mounts {
            None
        } else {
            Some(&working_dir)
        };

        let mut app = spin_loader::from_file(manifest_file, asset_dst, &bindle_connection).await?;

        // Apply --env to component environments
        if !self.env.is_empty() {
            for component in app.components.iter_mut() {
                component.wasm.environment.extend(self.env.iter().cloned());
            }
        }

        let command = trigger_command_from_app(&app)?;

        let exec_opts = if self.help {
            TriggerExecOpts::NoApp
        } else {
            TriggerExecOpts::Local { app, working_dir }
        };

        Ok((command, exec_opts))
    }

    async fn prepare_app_from_oci(
        &self,
        reference: &str,
        working_dir: PathBuf,
    ) -> Result<(Vec<String>, TriggerExecOpts)> {
        let mut client = spin_publish::oci::client::Client::new(self.insecure, None)
            .await
            .context("cannot create registry client")?;

        client
            .pull(reference)
            .await
            .context("cannot pull Spin application from registry")?;

        let app_path = client
            .cache
            .lockfile_path(&reference)
            .await
            .context("cannot get path to spin.lock")?;

        let locked_app: LockedApp = serde_json::from_slice(&tokio::fs::read(&app_path).await?)?;
        self.prepare_locked_app(locked_app, working_dir).await
    }

    async fn prepare_app_from_bindle(
        &self,
        bindle_id: &str,
        working_dir: PathBuf,
    ) -> Result<(Vec<String>, TriggerExecOpts)> {
        let app = match &self.server {
            Some(server) => {
                assert!(!self.direct_mounts);
                spin_loader::from_bindle(bindle_id, server, &working_dir).await?
            }
            None => bail!("Loading from a bindle requires a Bindle server URL"),
        };

        let locked_app = spin_trigger::locked::build_locked_app(app, &working_dir)?;
        self.prepare_locked_app(locked_app, working_dir).await
    }
}

enum WorkingDirectory {
    Given(PathBuf),
    Temporary(TempDir),
}

impl WorkingDirectory {
    fn path(&self) -> &Path {
        match self {
            Self::Given(p) => p,
            Self::Temporary(t) => t.path(),
        }
    }
}

// Parse the environment variables passed in `key=value` pairs.
fn parse_env_var(s: &str) -> Result<(String, String)> {
    let parts: Vec<_> = s.splitn(2, '=').collect();
    if parts.len() != 2 {
        bail!("Environment variable must be of the form `key=value`");
    }
    Ok((parts[0].to_owned(), parts[1].to_owned()))
}

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

#[allow(clippy::large_enum_variant)] // The large variant is the common case and really this is equivalent to an Option
enum TriggerExecOpts {
    NoApp,
    Local {
        app: Application,
        working_dir: PathBuf,
    },
    Remote {
        locked_url: String,
        working_dir: PathBuf,
        from_registry: bool,
    },
}

fn trigger_command(trigger_type: &str) -> Vec<String> {
    vec!["trigger".to_owned(), trigger_type.to_owned()]
}

fn trigger_command_from_app(app: &Application) -> Result<Vec<String>> {
    match &app.info.trigger {
        ApplicationTrigger::Http(_) => Ok(trigger_command("http")),
        ApplicationTrigger::Redis(_) => Ok(trigger_command("redis")),
        ApplicationTrigger::External(cfg) => {
            resolve_trigger_plugin(cfg.trigger_type()).map(|p| vec![p])
        }
    }
}

fn trigger_command_from_locked_app(locked_app: &LockedApp) -> Result<Vec<String>> {
    let trigger_metadata = locked_app
        .metadata
        .get("trigger")
        .cloned()
        .ok_or_else(|| anyhow!("missing trigger metadata in locked application"))?;

    let trigger_info: ApplicationTrigger = serde_json::from_value(trigger_metadata)
        .context("deserializing trigger type from locked application")?;

    match trigger_info {
        ApplicationTrigger::Http(_) => Ok(trigger_command("http")),
        ApplicationTrigger::Redis(_) => Ok(trigger_command("redis")),
        ApplicationTrigger::External(cfg) => {
            resolve_trigger_plugin(cfg.trigger_type()).map(|p| vec![p])
        }
    }
}
