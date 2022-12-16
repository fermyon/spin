use std::{
    ffi::OsString,
    fmt::Debug,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Context, Result};
use clap::{CommandFactory, Parser};
use reqwest::Url;
use spin_loader::bindle::BindleConnectionInfo;
use spin_manifest::ApplicationTrigger;
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
        )]
    pub app: Option<PathBuf>,

    /// ID of application bindle.
    #[clap(
            name = BINDLE_ID_OPT,
            short = 'b',
            long = "bindle",
            conflicts_with = APP_CONFIG_FILE_OPT,
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
        requires = BINDLE_PASSWORD
    )]
    pub bindle_username: Option<String>,

    /// Basic http auth password for the bindle server
    #[clap(
        name = BINDLE_PASSWORD,
        long = "bindle-password",
        env = BINDLE_PASSWORD,
        requires = BINDLE_USERNAME
    )]
    pub bindle_password: Option<String>,

    /// Ignore server certificate errors from bindle server
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

    /// Only run a subset of the components
    #[clap(long = "component", short = 'c')]
    pub include_components: Vec<String>,
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
        let working_dir_holder = match &self.tmp {
            None => WorkingDirectory::Temporary(tempfile::tempdir()?),
            Some(d) => WorkingDirectory::Given(d.to_owned()),
        };
        let working_dir = working_dir_holder.path().canonicalize()?;

        let mut app = match (&self.app, &self.bindle) {
            (app, None) => {
                let manifest_file = app
                    .as_deref()
                    .unwrap_or_else(|| DEFAULT_MANIFEST_FILE.as_ref());
                let bindle_connection = self.bindle_connection();
                
                let asset_dst = if self.direct_mounts {
                    None
                } else {
                    Some(&working_dir)
                };
                
                spin_loader::from_file(
                    manifest_file,
                    asset_dst,
                    &bindle_connection,
                    self.include_components,
                )
                .await?

            }
            (None, Some(bindle)) => match &self.server {
                Some(server) => {
                    assert!(!self.direct_mounts);

                    spin_loader::from_bindle(bindle, server, &working_dir).await?
                }
                _ => bail!("Loading from a bindle requires a Bindle server URL"),
            },
            (Some(_), Some(_)) => bail!("Specify only one of app file or bindle ID"),
        };

        // Apply --env to component environments
        if !self.env.is_empty() {
            for component in app.components.iter_mut() {
                component.wasm.environment.extend(self.env.iter().cloned());
            }
        }

        let trigger_type = match app.info.trigger {
            ApplicationTrigger::Http(_) => "http",
            ApplicationTrigger::Redis(_) => "redis",
        };

        // The docs for `current_exe` warn that this may be insecure because it could be executed
        // via hard-link. I think it should be fine as long as we aren't `setuid`ing this binary.
        let mut cmd = std::process::Command::new(std::env::current_exe().unwrap());
        cmd.arg("trigger")
            .arg(trigger_type)
            .env(SPIN_WORKING_DIR, &working_dir);

        if self.help {
            cmd.arg("--help-args-only");
        } else {
            let locked_url = self.write_locked_app(app, &working_dir)?;
            cmd.env(SPIN_LOCKED_URL, locked_url)
                .args(&self.trigger_args);
        };

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
