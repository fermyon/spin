use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use vaultrs::{
    client::{VaultClient, VaultClientSettingsBuilder},
    kv2,
};

use crate::{Key, Provider};

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
        prefix: Option<String>,
    ) -> Self {
        Self {
            url: url.into(),
            token: token.into(),
            mount: mount.into(),
            prefix,
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
            Some(prefix) => format!("{}/{}", prefix, key.0),
            None => key.0.to_string(),
        };
        let secret: Secret = kv2::read(&client, &self.mount, &path).await?;
        Ok(Some(secret.value))
    }
}
