use std::{
    ffi::OsString,
    fmt::Debug,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use clap::{CommandFactory, Parser};
use spin_loader::bindle::BindleConnectionInfo;
use spin_manifest::ApplicationTrigger;
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

    /// Temporary directory for the static assets of the components.
    #[clap(long = "temp")]
    pub tmp: Option<PathBuf>,

    /// All other args, to be passed through to the trigger
    pub trigger_args: Vec<OsString>,
}

impl UpCommand {
    pub async fn run(self) -> Result<()> {
        let help = self.help;
        self.run_inner().await.or_else(|err| {
            if help {
                tracing::warn!("Error resolving trigger-specific help: {}", err);
                Self::command().print_help()?;
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
        let working_dir = working_dir_holder.path();

        let app = match (&self.app, &self.bindle) {
            (app, None) => {
                let manifest_file = app
                    .as_deref()
                    .unwrap_or_else(|| DEFAULT_MANIFEST_FILE.as_ref());
                let bindle_connection = self.bindle_connection();
                spin_loader::from_file(manifest_file, working_dir, &bindle_connection).await?
            }
            (None, Some(bindle)) => match &self.server {
                Some(server) => spin_loader::from_bindle(bindle, server, working_dir).await?,
                _ => bail!("Loading from a bindle requires a Bindle server URL"),
            },
            (Some(_), Some(_)) => bail!("Specify only one of app file or bindle ID"),
        };

        let manifest_url = match app.info.origin {
            spin_manifest::ApplicationOrigin::File(path) => {
                format!("file://{}", path.canonicalize()?.to_string_lossy())
            }
            spin_manifest::ApplicationOrigin::Bindle { id, server } => {
                format!("bindle+{}?id={}", server, id)
            }
        };

        let trigger_type = match app.info.trigger {
            ApplicationTrigger::Http(_) => "http",
            ApplicationTrigger::Redis(_) => "redis",
        };

        let trigger_args = if self.help {
            vec![OsString::from("--help")]
        } else {
            self.trigger_args
        };

        // The docs for `current_exe` warn that this may be insecure because it could be executed
        // via hard-link. I think it should be fine as long as we aren't `setuid`ing this binary.
        let mut cmd = std::process::Command::new(std::env::current_exe().unwrap());
        cmd.arg("trigger")
            .env("SPIN_WORKING_DIR", working_dir)
            .env("SPIN_MANIFEST_URL", manifest_url)
            .env("SPIN_TRIGGER_TYPE", trigger_type)
            .arg(trigger_type)
            .args(trigger_args);

        if let Some(bindle_server) = self.server {
            cmd.env(BINDLE_URL_ENV, bindle_server);
        }

        tracing::trace!("Running trigger executor: {:?}", cmd);

        let status = cmd.status().context("Failed to execute trigger")?;
        if status.success() {
            Ok(())
        } else {
            bail!(status);
        }
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
