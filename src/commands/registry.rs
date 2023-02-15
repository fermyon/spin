use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::opts::*;

/// Commands for working with OCI registries to distribute applications.
/// The set of commands for OCI is EXPERIMENTAL, and may change in future versions of Spin.
/// Currently, the OCI commands are reusing the credentials from ~/.docker/config.json to
/// authenticate to registries.
#[derive(Subcommand, Debug)]
pub enum RegistryCommands {
    /// Push a Spin application to an OCI registry.
    Push(Push),
    /// Pull a Spin application from an OCI registry.
    Pull(Pull),
}

impl RegistryCommands {
    pub async fn run(self) -> Result<()> {
        match self {
            RegistryCommands::Push(cmd) => cmd.run().await,
            RegistryCommands::Pull(cmd) => cmd.run().await,
        }
    }
}

#[derive(Parser, Debug)]
pub struct Push {
    /// Path to spin.toml
    #[clap(
        name = APP_CONFIG_FILE_OPT,
        short = 'f',
        long = "file",
    )]
    pub app: Option<PathBuf>,

    /// Ignore server certificate errors
    #[clap(
        name = INSECURE_OPT,
        short = 'k',
        long = "insecure",
        takes_value = false,
    )]
    pub insecure: bool,

    /// Reference of the Spin application
    #[clap()]
    pub reference: String,
}

impl Push {
    pub async fn run(self) -> Result<()> {
        let app_file = self
            .app
            .as_deref()
            .unwrap_or_else(|| DEFAULT_MANIFEST_FILE.as_ref());

        let dir = tempfile::tempdir()?;
        let app = spin_loader::local::from_file(&app_file, Some(dir.path()), &None).await?;

        let mut client = spin_publish::oci::client::Client::new(self.insecure, None).await?;
        client.push(&app, &self.reference).await?;
        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct Pull {
    /// Ignore server certificate errors
    #[clap(
        name = INSECURE_OPT,
        short = 'k',
        long = "insecure",
        takes_value = false,
    )]
    pub insecure: bool,

    /// Reference of the Spin application
    #[clap()]
    pub reference: String,
}

impl Pull {
    /// Pull a Spin application from an OCI registry
    pub async fn run(self) -> Result<()> {
        let mut client = spin_publish::oci::client::Client::new(self.insecure, None).await?;
        client.pull(&self.reference).await?;

        Ok(())
    }
}
