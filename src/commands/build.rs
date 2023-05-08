use std::{ffi::OsString, path::PathBuf};

use anyhow::Result;
use clap::Parser;

use crate::opts::{APP_MANIFEST_FILE_OPT, BUILD_UP_OPT, DEFAULT_MANIFEST_FILE};

use super::up::UpCommand;

/// Run the build command for each component.
#[derive(Parser, Debug)]
#[clap(about = "Build the Spin application", allow_hyphen_values = true)]
pub struct BuildCommand {
    /// The application to build. This may be a manifest (spin.toml) file, or a
    /// directory containing a spin.toml file.
    /// If omitted, it defaults to "spin.toml".
    #[clap(
        name = APP_MANIFEST_FILE_OPT,
        short = 'f',
        long = "from",
        alias = "file",
        default_value = DEFAULT_MANIFEST_FILE
    )]
    pub app_source: PathBuf,

    /// Run the application after building.
    #[clap(name = BUILD_UP_OPT, short = 'u', long = "up")]
    pub up: bool,

    #[clap(requires = BUILD_UP_OPT)]
    pub up_args: Vec<OsString>,
}

impl BuildCommand {
    pub async fn run(self) -> Result<()> {
        let manifest_file = crate::manifest::resolve_file_path(&self.app_source)?;
        spin_build::build(&manifest_file).await?;

        if self.up {
            let mut cmd = UpCommand::parse_from(
                std::iter::once(OsString::from(format!(
                    "{} up",
                    std::env::args().next().unwrap()
                )))
                .chain(self.up_args),
            );
            cmd.file_source = Some(manifest_file);
            cmd.run().await
        } else {
            Ok(())
        }
    }
}
