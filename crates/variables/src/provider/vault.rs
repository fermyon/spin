use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
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
    retry_limit: u8, // specifically *re*-tries; e.g. 0 = try but do not retry, 1 = try then retry at most once
}

impl VaultProvider {
    pub fn new(
        url: impl Into<String>,
        token: impl Into<String>,
        mount: impl Into<String>,
        prefix: Option<impl Into<String>>,
        retry_limit: Option<u8>,
    ) -> Self {
        Self {
            url: url.into(),
            token: token.into(),
            mount: mount.into(),
            prefix: prefix.map(Into::into),
            retry_limit: retry_limit.unwrap_or_default(),
        }
    }
}

#[derive(Deserialize, Serialize)]
struct Secret {
    value: String,
}

#[async_trait]
impl Provider for VaultProvider {
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

        let mut retry_count = 0;
        loop {
            match kv2::read::<Secret>(&client, &self.mount, &path).await {
                Ok(secret) => return Ok(Some(secret.value)),
                // Vault doesn't have this entry so pass along the chain
                Err(ClientError::APIError { code: 404, .. }) => return Ok(None),
                // Other Vault error so retry, or if at retry limit bail rather than looking elsewhere
                Err(e) => {
                    if retry_count >= self.retry_limit {
                        return Err(e).context("Failed to check Vault for config");
                    } else {
                        tracing::warn!("Failed to check Vault for config - retrying ({e})");
                        retry_count += 1;
                        continue;
                    }
                }
            }
        }
    }
}
