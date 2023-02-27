use std::{ffi::OsString, path::PathBuf};

use anyhow::Result;
use clap::Parser;

use crate::opts::{APP_MANIFEST_FILE_OPT, BUILD_UP_OPT, DEFAULT_MANIFEST_FILE};

use super::up::UpCommand;

/// Run the build command for each component.
#[derive(Parser, Debug)]
#[clap(about = "Build the Spin application", allow_hyphen_values = true)]
pub struct BuildCommand {
    /// Path to application manifest. The default is "spin.toml".
    #[clap(
        name = APP_MANIFEST_FILE_OPT,
        short = 'f',
        long = "from",
        alias = "file",
    )]
    pub app: Option<PathBuf>,

    /// Run the application after building.
    #[clap(name = BUILD_UP_OPT, short = 'u', long = "up")]
    pub up: bool,

    #[clap(requires = BUILD_UP_OPT)]
    pub up_args: Vec<OsString>,
}

impl BuildCommand {
    pub async fn run(self) -> Result<()> {
        let manifest_file = self
            .app
            .as_deref()
            .unwrap_or_else(|| DEFAULT_MANIFEST_FILE.as_ref());
        spin_build::build(manifest_file).await?;

        if self.up {
            let mut cmd = UpCommand::parse_from(
                std::iter::once(OsString::from(format!(
                    "{} up",
                    std::env::args().next().unwrap()
                )))
                .chain(self.up_args),
            );
            cmd.file_source = Some(manifest_file.into());
            cmd.run().await
        } else {
            Ok(())
        }
    }
}
