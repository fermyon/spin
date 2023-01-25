use crate::{args::manifest_file::ManifestFile, commands::up::UpCommand, opts::BINDLE_URL_ENV};
use anyhow::Result;
use clap::{error::ErrorKind, Args, Command, CommandFactory, Error, FromArgMatches, Id};
use spin_manifest::Application;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub enum AppSource {
    Local(LocalSource),
    Bindle(BindleSource),
}

impl Default for AppSource {
    fn default() -> Self {
        Self::Local(LocalSource::default())
    }
}

impl AppSource {
    pub async fn load(&self, working_dir: &PathBuf) -> Result<Application, Error> {
        match self {
            AppSource::Local(app) => {
                spin_loader::from_file(
                    &app.file,
                    if app.direct_mounts {
                        Some(&working_dir)
                    } else {
                        None
                    },
                    &None,
                )
                .await
            }
            AppSource::Bindle(bindle) => {
                spin_loader::from_bindle(
                    bindle.bindle.as_str(),
                    bindle.bindle_server.as_str(),
                    &working_dir,
                )
                .await
            }
        }
        .map_err(|_| Error::new(ErrorKind::Io).with_cmd(&UpCommand::command()))
    }
}

impl FromArgMatches for AppSource {
    fn from_arg_matches(matches: &clap::ArgMatches) -> Result<Self, Error> {
        let bindle = BindleSource::from_arg_matches(matches);
        let local = LocalSource::from_arg_matches(matches);

        if matches.contains_id("BindleOptions") && matches.contains_id("LocalOptions") {
            return Err(Error::new(ErrorKind::ArgumentConflict));
        }

        match (local, bindle) {
            (Err(local), Err(_)) => return Err(local),
            (Ok(local), _) => Ok(Self::Local(local)),
            (_, Ok(bindle)) => Ok(Self::Bindle(bindle)),
        }
    }

    fn update_from_arg_matches(&mut self, matches: &clap::ArgMatches) -> Result<(), Error> {
        Ok(*self = Self::from_arg_matches(matches)?)
    }
}

impl Args for AppSource {
    fn group_id() -> Option<Id> {
        None
    }
    fn augment_args(cmd: Command) -> Command {
        BindleSource::augment_args(LocalSource::augment_args(cmd))
    }

    fn augment_args_for_update(cmd: Command) -> Command {
        BindleSource::augment_args_for_update(LocalSource::augment_args_for_update(cmd))
    }
}

#[derive(Args, Clone, Debug, Default)]
#[command(next_help_heading = "Local Options")]
pub struct LocalSource {
    /// Path to spin.toml
    #[arg(long, short, default_value_t = ManifestFile::default(), required = false)]
    file: ManifestFile,

    /// For local apps with directory mounts and no excluded files, mount them directly instead of using a temporary
    /// directory.
    ///
    /// This allows you to update the assets on the host filesystem such that the updates are visible to the guest
    /// without a restart.  This cannot be used with bindle apps or apps which use file patterns and/or exclusions.
    #[arg(long)]
    direct_mounts: bool,
}

impl LocalSource {
    pub fn new(file: ManifestFile, direct_mounts: bool) -> Self {
        Self {
            file,
            direct_mounts,
        }
    }
}

#[derive(Args, Clone, Debug, Default)]
#[command(next_help_heading = "Bindle Options")]
pub struct BindleSource {
    #[arg(long, short, required = false)]
    bindle: String,
    /// URL of bindle server.
    #[arg(long, env = BINDLE_URL_ENV, required = false)]
    bindle_server: String,

    /// Basic http auth username for the bindle server
    #[arg(long, env, requires = "bindle_password")]
    bindle_username: Option<String>,
    /// Basic http auth password for the bindle server
    #[arg(long, env, requires = "bindle_username")]
    bindle_password: Option<String>,

    /// Ignore server certificate errors from bindle server
    #[arg(short = 'k', long)]
    insecure: bool,
}
