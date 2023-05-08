pub mod commands;
pub mod manifest;
pub(crate) mod opts;
mod watch_filter;
mod watch_state;

use anyhow::{anyhow, Result};
use semver::BuildMetadata;
use spin_bindle::PublishError;
use std::path::Path;

pub use crate::opts::HELP_ARGS_ONLY_TRIGGER_TYPE;

pub(crate) fn push_all_failed_msg(path: &Path, server_url: &str) -> String {
    format!(
        "Failed to push bindle from '{}' to the server at '{}'",
        path.display(),
        server_url
    )
}

pub(crate) fn wrap_prepare_bindle_error(err: PublishError) -> anyhow::Error {
    match err {
        PublishError::MissingBuildArtifact(_) => {
            anyhow!("{}\n\nPlease try to run `spin build` first", err)
        }
        e => anyhow!(e),
    }
}

pub(crate) fn parse_buildinfo(buildinfo: &str) -> Result<BuildMetadata> {
    Ok(BuildMetadata::new(buildinfo)?)
}
