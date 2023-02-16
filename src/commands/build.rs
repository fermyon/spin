use std::{ffi::OsString, path::PathBuf};

use anyhow::{bail, Result};
use clap::Parser;

use crate::opts::{APP_CONFIG_FILE_OPT, BUILD_UP_OPT, DEFAULT_MANIFEST_FILE};

use super::up::UpCommand;

/// Run the build command for each component.
#[derive(Parser, Debug)]
#[clap(about = "Build the Spin application", allow_hyphen_values = true)]
pub struct BuildCommand {
    /// Path to application manifest. The default is "spin.toml".
    #[clap(name = "APPLICATION")]
    pub app_source: Option<PathBuf>,

    /// Path to spin.toml.
    #[clap(
        hide = true,
        name = APP_CONFIG_FILE_OPT,
        short = 'f',
        long = "file",
        conflicts_with = "APPLICATION"
    )]
    pub file_source: Option<PathBuf>,

    /// Run the application after building.
    #[clap(name = BUILD_UP_OPT, short = 'u', long = "up")]
    pub up: bool,

    #[clap(requires = BUILD_UP_OPT)]
    pub up_args: Vec<OsString>,
}

impl BuildCommand {
    pub async fn run(self) -> Result<()> {
        let manifest_file = match (self.app_source.as_deref(), self.file_source.as_deref()) {
            (None, None) => DEFAULT_MANIFEST_FILE.as_ref(),
            (Some(p), None) => p,
            (None, Some(p)) => p,
            (Some(_), Some(_)) => bail!("Cannot specify more than one application"),
        };

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
