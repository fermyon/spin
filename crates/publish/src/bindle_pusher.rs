#![deny(missing_docs)]

use std::path::Path;

use anyhow::{Context, Result};
use bindle::{standalone::StandaloneRead, Id};

/// Pushes a standalone bindle to a Bindle server.
pub async fn push_all(
    path: impl AsRef<Path>,
    bindle_id: &Id,
    bindle_connection_info: crate::BindleConnectionInfo,
) -> Result<()> {
    let reader = StandaloneRead::new(&path, bindle_id).await?;
    let client = &bindle_connection_info.client().with_context(|| {
        format!(
            "Failed to create a bindle client for server '{}'",
            &bindle_connection_info.base_url
        )
    })?;
    reader
        .push(client)
        .await
        .with_context(|| push_failed_msg(path, &bindle_connection_info.base_url))
}

fn push_failed_msg(path: impl AsRef<Path>, server_url: &str) -> String {
    format!(
        "Failed to push bindle from '{}' to server at '{}'",
        path.as_ref().display(),
        server_url
    )
}
