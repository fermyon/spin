pub mod commands;

use anyhow::Context;
use bindle::client::Client as BindleClient;
use bindle::client::ClientBuilder as BindleClientBuilder;
use semver::{BuildMetadata, Error};
use spin_loader::bindle::BindleTokenManager;
use std::path::Path;

pub(crate) fn app_dir(app_file: impl AsRef<Path>) -> Result<std::path::PathBuf, anyhow::Error> {
    let path_buf = app_file
        .as_ref()
        .parent()
        .ok_or_else(|| {
            anyhow::anyhow!(
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

pub(crate) fn create_bindle_client(
    insecure: bool,
    bindle_server_url: &str,
) -> Result<BindleClient<BindleTokenManager>, anyhow::Error> {
    BindleClientBuilder::default()
        .danger_accept_invalid_certs(insecure)
        .build(
            bindle_server_url,
            // TODO: pick up auth options from the command line
            BindleTokenManager::NoToken(bindle::client::tokens::NoToken),
        )
        .with_context(|| {
            format!(
                "Failed to create client for bindle server '{}'",
                bindle_server_url,
            )
        })
}

pub(crate) fn parse_buildinfo(buildinfo: &str) -> Result<BuildMetadata, Error> {
    BuildMetadata::new(buildinfo)
}
