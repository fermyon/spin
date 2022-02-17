#![deny(missing_docs)]

use std::path::Path;

use anyhow::{Context, Result};
use bindle::standalone::StandaloneRead;

type BindleClient = bindle::client::Client<spin_loader::bindle::BindleTokenManager>;

/// Pushes a standalone bindle to a Bindle server.
pub async fn push_all(
    path: impl AsRef<Path>,
    bindle_id: &bindle::Id,
    client: &BindleClient,
    server_url: &str,
) -> Result<()> {
    let reader = StandaloneRead::new(&path, bindle_id).await?;
    reader
        .push(client)
        .await
        .with_context(|| push_failed_msg(path, server_url))
}

fn push_failed_msg(path: impl AsRef<Path>, server_url: &str) -> String {
    format!(
        "Failed to push bindle from '{}' to server at '{}'",
        path.as_ref().display(),
        server_url
    )
}
