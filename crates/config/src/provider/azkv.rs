use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use azure_core::StatusCode;
use azure_identity::{ClientSecretCredential, TokenCredentialOptions};
use azure_security_keyvault::KeyvaultClient;

use crate::{Key, Provider};

/// A config Provider that uses Azure Key Vault.
#[derive(Debug)]
pub struct AzureKeyVaultProvider {
    client_id: String,
    client_secret: String,
    tenant_id: String,
    url: String,
}

impl AzureKeyVaultProvider {
    pub fn new(
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        tenant_id: impl Into<String>,
        url: impl Into<String>,
    ) -> Self {
        Self {
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            tenant_id: tenant_id.into(),
            url: url.into(),
        }
    }
}

#[async_trait]
impl Provider for AzureKeyVaultProvider {
    async fn get(&self, key: &Key) -> Result<Option<String>> {
        let http_client = azure_core::new_http_client();
        let creds = ClientSecretCredential::new(
            http_client,
            self.tenant_id.clone(),
            self.client_id.clone(),
            self.client_secret.clone(),
            TokenCredentialOptions::default(),
        );

        let kv_client = KeyvaultClient::new(&self.url, Arc::new(creds))?;
        let secret_client = kv_client.secret_client();

        match secret_client.get(key.0).await {
            Ok(secret) => Ok(Some(secret.value)),
            Err(err) => {
                let Some(http_err) = err.as_http_error() else {
                    return Err(err).context("Failed to check Azure Key Vault for config");
                };
                let StatusCode::NotFound = http_err.status() else {
                    return Err(err).context("Failed to check Azure Key Vault for config");
                };
                Ok(None)
            }
        }
    }
}
