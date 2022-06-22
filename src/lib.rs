pub mod commands;
pub(crate) mod opts;
mod sloth;

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use semver::BuildMetadata;

pub(crate) fn app_dir(app_file: impl AsRef<Path>) -> Result<PathBuf> {
    let path_buf = app_file
        .as_ref()
        .parent()
        .ok_or_else(|| {
            anyhow!(
                "Failed to get containing directory for app file '{}'",
                app_file.as_ref().display()
            )
        })?
        .to_owned();
    Ok(path_buf)
}

pub(crate) fn write_failed_msg(bindle_id: &bindle::Id, dest_dir: &Path) -> String {
    format!(
        "Failed to write bindle '{}' to {}",
        bindle_id,
        dest_dir.display()
    )
}

pub(crate) fn parse_buildinfo(buildinfo: &str) -> Result<BuildMetadata> {
    Ok(BuildMetadata::new(buildinfo)?)
}
