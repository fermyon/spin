use std::path::PathBuf;

use crate::{
    commands::up::{UpCommand, UpOpts},
    opts::{APP_CONFIG_FILE_OPT, BUILD_UP_OPT, DEFAULT_MANIFEST_FILE},
};
use anyhow::Result;
use spin_loader::local::{config::RawAppManifestAnyVersion, raw_manifest_from_file};
use structopt::{clap::AppSettings, StructOpt};

/// Run the build command for each component.
#[derive(StructOpt, Debug)]
#[structopt(
    about = "Build the Spin application",
    global_settings = &[AppSettings::ColoredHelp]
)]
pub struct BuildCommand {
    #[structopt(flatten)]
    pub opts: UpOpts,

    /// Path to spin.toml.
    #[structopt(
            name = APP_CONFIG_FILE_OPT,
            short = "f",
            long = "file",
        )]
    pub app: Option<PathBuf>,

    /// Run the application after building.
    #[structopt(name = BUILD_UP_OPT, short = "u", long = "up")]
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
