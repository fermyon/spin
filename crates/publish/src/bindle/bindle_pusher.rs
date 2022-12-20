#![deny(missing_docs)]

use super::{PublishError, PublishResult};
use bindle::{standalone::StandaloneRead, Id};
use std::path::Path;

/// Pushes a standalone bindle to a Bindle server.
pub async fn push_all(
    path: impl AsRef<Path>,
    bindle_id: &Id,
    bindle_connection_info: spin_loader::bindle::BindleConnectionInfo,
) -> PublishResult<()> {
    let reader = StandaloneRead::new(&path, bindle_id).await?;
    let client = &bindle_connection_info.client()?;

    if client.get_yanked_invoice(bindle_id).await.is_ok() {
        return Err(PublishError::BindleAlreadyExists(bindle_id.to_string()));
    }

    reader.push(client).await?;

    Ok(())
}
