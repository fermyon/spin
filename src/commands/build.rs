use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use spin_loader::local::{config::RawAppManifestAnyVersion, raw_manifest_from_file};

use crate::{
    commands::up::{UpCommand, UpOpts},
    opts::{APP_CONFIG_FILE_OPT, BUILD_UP_OPT, DEFAULT_MANIFEST_FILE},
};

/// Run the build command for each component.
#[derive(Parser, Debug)]
#[clap(about = "Build the Spin application")]
pub struct BuildCommand {
    #[clap(flatten)]
    pub opts: UpOpts,

    /// Path to spin.toml.
    #[clap(
            name = APP_CONFIG_FILE_OPT,
            short = 'f',
            long = "file",
        )]
    pub app: Option<PathBuf>,

    /// Run the application after building.
    #[clap(name = BUILD_UP_OPT, short = 'u', long = "up")]
    pub up: bool,
}

impl BuildCommand {
    pub async fn run(self) -> Result<()> {
        let manifest_file = self
            .app
            .as_deref()
            .unwrap_or_else(|| DEFAULT_MANIFEST_FILE.as_ref());
        let RawAppManifestAnyVersion::V1(app) = raw_manifest_from_file(&manifest_file).await?;

        spin_build::build(app, manifest_file).await?;

        if self.up {
            let cmd = UpCommand {
                app: Some(manifest_file.into()),
                opts: self.opts,
                bindle: None,
                server: None,
            };

            cmd.run().await
        } else {
            Ok(())
        }
    }
}
