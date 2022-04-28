use crate::{opts::*, parse_buildinfo};
use anyhow::{Context, Result};
use semver::BuildMetadata;
use std::path::PathBuf;
use structopt::StructOpt;

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

    /// Basic http auth username for the bindle server
    #[structopt(
        name = BINDLE_HTTP_USER,
        long = "bindle-http-user",
        env = BINDLE_HTTP_USER,
        requires(BINDLE_HTTP_PASSWORD)
    )]
    pub bindle_http_user: Option<String>,

    /// Basic http auth password for the bindle server
    #[structopt(
        name = BINDLE_HTTP_PASSWORD,
        long = "bindle-http-password",
        env = BINDLE_HTTP_PASSWORD,
        requires(BINDLE_HTTP_USER)
    )]
    pub bindle_http_password: Option<String>,

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
        let bindle_connection_info = spin_publish::BindleConnectionInfo::new(
            &self.bindle_server_url,
            self.insecure,
            self.bindle_http_user,
            self.bindle_http_password,
        );

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

        spin_publish::push_all(&dest_dir, bindle_id, bindle_connection_info)
            .await
            .context("Failed to push bindle to server")?;

        println!("pushed: {}", bindle_id);
        Ok(())
    }
}
