pub mod commands;
pub(crate) mod opts;
mod sloth;

use anyhow::Result;
use semver::BuildMetadata;

pub(crate) fn parse_buildinfo(buildinfo: &str) -> Result<BuildMetadata> {
    Ok(BuildMetadata::new(buildinfo)?)
}
