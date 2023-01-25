use std::path::PathBuf;

use anyhow::{Context, Result};
use async_trait::async_trait;
use clap::{Parser, Subcommand};
use semver::BuildMetadata;
use spin_loader::bindle::BindleConnectionInfo;

use crate::{opts::*, parse_buildinfo, sloth::warn_if_slow_response, args::manifest_file::ManifestFile, dispatch::Dispatch};

use crate::dispatch::Runner;

/// Commands for publishing applications as bindles.
#[derive(Subcommand, Debug)]
pub enum BindleCommands {
    /// Create a standalone bindle for subsequent publication.
    Prepare(Prepare),

    /// Publish an application as a bindle.
    Push(Push),
}

#[async_trait(?Send)]
impl Dispatch for BindleCommands {
    async fn run(&self) -> Result<()> {
        match self {
            Self::Prepare(cmd) => cmd.run().await,
            Self::Push(cmd) => cmd.run().await,
        }
    }
}

/// Create a standalone bindle for subsequent publication.
#[derive(Parser, Debug)]
pub struct Prepare {
    /// Path to spin.toml
    #[arg(
        short = 'f',
        long = "file",
        id = APP_CONFIG_FILE_OPT
    )]
    pub app: Option<PathBuf>,

    /// Build metadata to append to the bindle version
    #[arg(
        long = "buildinfo",
        id = BUILDINFO_OPT,
        value_parser = parse_buildinfo,
    )]
    pub buildinfo: Option<BuildMetadata>,

    /// Path to create standalone bindle.
    #[arg(
        short = 'd',
        long = "staging-dir",
        id = STAGING_DIR_OPT
    )]
    pub staging_dir: PathBuf,
}

/// Publish an application as a bindle.
#[derive(Parser, Debug)]
pub struct Push {
    /// Path to spin.toml
    #[arg(long, short, default_value_t = Default::default())]
    pub file: ManifestFile,

    /// Build metadata to append to the bindle version
    #[arg(
        long = "buildinfo",
        id = BUILDINFO_OPT,
        value_parser = parse_buildinfo,
    )]
    pub buildinfo: Option<BuildMetadata>,

    /// Path to assemble the bindle before pushing (defaults to
    /// temporary directory).
    #[arg(
        short = 'd',
        long = "staging-dir",
        id = STAGING_DIR_OPT
    )]
    pub staging_dir: Option<PathBuf>,

    /// URL of bindle server
    #[arg(
        long = "bindle-server",
        id = BINDLE_SERVER_URL_OPT,
        env = BINDLE_URL_ENV,
    )]
    pub bindle_server_url: String,

    /// Basic http auth username for the bindle server
    #[arg(
        long = "bindle-username",
        id = BINDLE_USERNAME,
        env = BINDLE_USERNAME,
        requires = BINDLE_PASSWORD
    )]
    pub bindle_username: Option<String>,

    /// Basic http auth password for the bindle server
    #[arg(
        long = "bindle-password",
        id = BINDLE_PASSWORD,
        env = BINDLE_PASSWORD,
        requires = BINDLE_USERNAME
    )]
    pub bindle_password: Option<String>,

    /// Ignore server certificate errors
    #[arg(
        short = 'k',
        long = "insecure",
        id = INSECURE_OPT,
    )]
    pub insecure: bool,
}

#[async_trait(?Send)]
impl Dispatch for Prepare {
    async fn run(&self) -> Result<()> {
        let app_file = self
            .app
            .as_deref()
            .unwrap_or_else(|| DEFAULT_MANIFEST_FILE.as_ref());

        let dest_dir = &self.staging_dir;
        let buildinfo = &self.buildinfo;
        let bindle_id = spin_publish::bindle::prepare_bindle(app_file, buildinfo.clone(), dest_dir)
            .await
            .map_err(crate::wrap_prepare_bindle_error)?;

        // We can't try to canonicalize it until the directory has been created
        let full_dest_dir =
            dunce::canonicalize(&self.staging_dir).unwrap_or_else(|_| dest_dir.clone());

        println!("id:      {}", bindle_id);
        #[rustfmt::skip]
        println!("command: bindle push -p {} {}", full_dest_dir.display(), bindle_id);
        Ok(())
    }
}

#[async_trait(?Send)]
impl Dispatch for Push {
    async fn run(&self) -> Result<()> {
        let app_file = self.file.canonicalize()?;
        let bindle_connection_info = BindleConnectionInfo::new(
            &self.bindle_server_url,
            self.insecure,
            self.bindle_username.clone(),
            self.bindle_password.clone(),
        );

        // TODO: only create this if not given a staging dir
        let temp_dir = tempfile::tempdir()?;

        let dest_dir = match &self.staging_dir {
            None => temp_dir.path(),
            Some(path) => path.as_path(),
        };

        let bindle_id =
            spin_publish::bindle::prepare_bindle(app_file, self.buildinfo.clone(), dest_dir)
                .await
                .map_err(crate::wrap_prepare_bindle_error)?;

        let _sloth_warning = warn_if_slow_response(format!(
            "Uploading application to {}",
            self.bindle_server_url
        ));

        spin_publish::bindle::push_all(&dest_dir, &bindle_id, bindle_connection_info.clone())
            .await
            .with_context(|| {
                crate::push_all_failed_msg(dest_dir, bindle_connection_info.base_url())
            })?;

        println!("pushed: {}", bindle_id);
        Ok(())
    }
}
