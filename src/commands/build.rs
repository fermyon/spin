use super::up::{UpCommand, Flag};
use crate::{args::manifest_file::ManifestFile, dispatch::Dispatch};
use crate::dispatch::Runner;
use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;

/// Build the Spin application
#[derive(Parser, Debug)]
#[command(subcommand_required=false)]
pub struct BuildCommand {
    /// Path to spin.toml.
    #[arg(long, short, default_value_t = ManifestFile::default(), required = false)]
    pub file: ManifestFile,
    #[command(subcommand)]
    pub up: Flag<UpCommand>,
}

#[async_trait(?Send)]
impl Dispatch for BuildCommand {
    async fn run(&self) -> Result<()> {
        let Self { file, up } = self;
        file.build().await?;
        up.run().await
    }
}
