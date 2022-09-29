#![deny(missing_docs)]

use anyhow::{Context, Result};
use bindle::{standalone::StandaloneRead, Id};
use std::path::Path;

/// Pushes a standalone bindle to a Bindle server.
pub(crate) async fn push_all(
    path: impl AsRef<Path>,
    bindle_id: &Id,
    bindle_connection_info: super::BindleConnectionInfo,
) -> Result<()> {
    let reader = StandaloneRead::new(&path, bindle_id).await?;
    let client = &bindle_connection_info.client().with_context(|| {
        format!(
            "Failed to create a bindle client for server '{}'",
            &bindle_connection_info.base_url
        )
    })?;

    if client.get_yanked_invoice(bindle_id).await.is_ok() {
        anyhow::bail!("Bindle {} already exists on the server", bindle_id);
    }

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
