use crate::parse_buildinfo;
use anyhow::{Context, Result};
use bindle::client::Client as BindleClient;
use bindle::client::ClientBuilder as BindleClientBuilder;
use semver::BuildMetadata;
use spin_loader::bindle::BindleTokenManager;
use std::path::PathBuf;
use structopt::StructOpt;

use crate::commands::up::DEFAULT_MANIFEST_FILE;

const APP_CONFIG_FILE_OPT: &str = "APP_CONFIG_FILE";
const BINDLE_SERVER_URL_OPT: &str = "BINDLE_SERVER_URL";
const BINDLE_URL_ENV: &str = "BINDLE_URL";
const BUILDINFO_OPT: &str = "BUILDINFO";
const INSECURE_OPT: &str = "INSECURE";
const STAGING_DIR_OPT: &str = "STAGING_DIR";

/// Commands for publishing applications as bindles.
#[derive(StructOpt, Debug)]
pub enum BindleCommands {
    /// Create a standalone bindle for subsequent publication.
    Prepare(Prepare),

    /// Publish an application as a bindle.
    Push(Push),
}

impl BindleCommands {
    pub async fn run(self) -> Result<()> {
        match self {
            Self::Prepare(cmd) => cmd.run().await,
            Self::Push(cmd) => cmd.run().await,
        }
    }
}

/// Create a standalone bindle for subsequent publication.
#[derive(StructOpt, Debug)]
pub struct Prepare {
    /// Path to spin.toml
    #[structopt(
        name = APP_CONFIG_FILE_OPT,
        short = "f",
        long = "file",
    )]
    pub app: Option<PathBuf>,

    /// Build metadata to append to the bindle version
    #[structopt(
        name = BUILDINFO_OPT,
        long = "buildinfo",
        parse(try_from_str = parse_buildinfo),
    )]
    pub buildinfo: Option<BuildMetadata>,

    /// Path to create standalone bindle.
    #[structopt(
        name = STAGING_DIR_OPT,
        long = "staging-dir",
        short = "-d",
    )]
    pub staging_dir: PathBuf,
}

/// Publish an application as a bindle.
#[derive(StructOpt, Debug)]
pub struct Push {
    /// Path to spin.toml
    #[structopt(
        name = APP_CONFIG_FILE_OPT,
        short = "f",
        long = "file",
    )]
    pub app: Option<PathBuf>,

    /// Build metadata to append to the bindle version
    #[structopt(
        name = BUILDINFO_OPT,
        long = "buildinfo",
        parse(try_from_str = parse_buildinfo),
    )]
    pub buildinfo: Option<BuildMetadata>,

    /// Path to assemble the bindle before pushing (defaults to
    /// temporary directory).
    #[structopt(
        name = STAGING_DIR_OPT,
        long = "staging-dir",
        short = "-d",
    )]
    pub staging_dir: Option<PathBuf>,

    /// URL of bindle server
    #[structopt(
        name = BINDLE_SERVER_URL_OPT,
        long = "bindle-server",
        env = BINDLE_URL_ENV,
    )]
    pub bindle_server_url: String,

    /// Ignore server certificate errors
    #[structopt(
        name = INSECURE_OPT,
        short = "k",
        long = "insecure",
        takes_value = false,
    )]
    pub insecure: bool,
}

impl Prepare {
    pub async fn run(self) -> Result<()> {
        let app_file = self
            .app
            .as_deref()
            .unwrap_or_else(|| DEFAULT_MANIFEST_FILE.as_ref());
        let source_dir = crate::app_dir(app_file)?;

        let dest_dir = &self.staging_dir;

        let (invoice, sources) = spin_publish::expand_manifest(app_file, self.buildinfo, &dest_dir)
            .await
            .with_context(|| format!("Failed to expand '{}' to a bindle", app_file.display()))?;

        let bindle_id = &invoice.bindle.id;

        spin_publish::write(&source_dir, &dest_dir, &invoice, &sources)
            .await
            .with_context(|| crate::write_failed_msg(bindle_id, dest_dir))?;

        // We can't try to canonicalise it until the directory has been created
        let full_dest_dir =
            dunce::canonicalize(&self.staging_dir).unwrap_or_else(|_| dest_dir.clone());

        println!("id:      {}", bindle_id);
        #[rustfmt::skip]
        println!("command: bindle push -p {} {}", full_dest_dir.display(), bindle_id);
        Ok(())
    }
}

impl Push {
    pub async fn run(self) -> Result<()> {
        let app_file = self
            .app
            .as_deref()
            .unwrap_or_else(|| DEFAULT_MANIFEST_FILE.as_ref());
        let source_dir = crate::app_dir(app_file)?;
        let client = self.create_bindle_client()?;

        // TODO: only create this if not given a staging dir
        let temp_dir = tempfile::tempdir()?;

        let dest_dir = match &self.staging_dir {
            None => temp_dir.path(),
            Some(path) => path.as_path(),
        };

        let (invoice, sources) = spin_publish::expand_manifest(app_file, self.buildinfo, &dest_dir)
            .await
            .with_context(|| format!("Failed to expand '{}' to a bindle", app_file.display()))?;

        let bindle_id = &invoice.bindle.id;

        spin_publish::write(&source_dir, &dest_dir, &invoice, &sources)
            .await
            .with_context(|| crate::write_failed_msg(bindle_id, dest_dir))?;

        spin_publish::push_all(&dest_dir, bindle_id, &client, &self.bindle_server_url)
            .await
            .context("Failed to push bindle to server")?;

        println!("pushed: {}", bindle_id);
        Ok(())
    }

    fn create_bindle_client(&self) -> Result<BindleClient<BindleTokenManager>> {
        BindleClientBuilder::default()
            .danger_accept_invalid_certs(self.insecure)
            .build(
                &self.bindle_server_url,
                // TODO: pick up auth options from the command line
                BindleTokenManager::NoToken(bindle::client::tokens::NoToken),
            )
            .with_context(|| {
                format!(
                    "Failed to create client for bindle server '{}'",
                    self.bindle_server_url
                )
            })
    }
}
