use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{instrument, Level};
use vaultrs::{
    client::{VaultClient, VaultClientSettingsBuilder},
    error::ClientError,
    kv2,
};

use spin_expressions::{Key, Provider};

/// A config Provider that uses HashiCorp Vault.
#[derive(Debug)]
pub struct VaultProvider {
    url: String,
    token: String,
    mount: String,
    prefix: Option<String>,
}

impl VaultProvider {
    pub fn new(
        url: impl Into<String>,
        token: impl Into<String>,
        mount: impl Into<String>,
        prefix: Option<impl Into<String>>,
    ) -> Self {
        Self {
            url: url.into(),
            token: token.into(),
            mount: mount.into(),
            prefix: prefix.map(Into::into),
        }
    }
}

#[derive(Deserialize, Serialize)]
struct Secret {
    value: String,
}

#[async_trait]
impl Provider for VaultProvider {
    #[instrument(name = "spin_variables.get_from_vault", skip(self), err(level = Level::INFO), fields(otel.kind = "client"))]
    async fn get(&self, key: &Key) -> Result<Option<String>> {
        let client = VaultClient::new(
            VaultClientSettingsBuilder::default()
                .address(&self.url)
                .token(&self.token)
                .build()?,
        )?;
        let path = match &self.prefix {
            Some(prefix) => format!("{}/{}", prefix, key.as_str()),
            None => key.as_str().to_string(),
        };
        match kv2::read::<Secret>(&client, &self.mount, &path).await {
            Ok(secret) => Ok(Some(secret.value)),
            // Vault doesn't have this entry so pass along the chain
            Err(ClientError::APIError { code: 404, .. }) => Ok(None),
            // Other Vault error so bail rather than looking elsewhere
            Err(e) => Err(e).context("Failed to check Vault for config"),
        }
    }
}
