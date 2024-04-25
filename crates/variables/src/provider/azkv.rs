use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use azure_core::Url;
use azure_security_keyvault::SecretClient;
use serde::Deserialize;
use spin_expressions::{Key, Provider};
use tracing::{instrument, Level};

#[derive(Debug)]
pub struct AzureKeyVaultProvider {
    client_id: String,
    client_secret: String,
    tenant_id: String,
    vault_url: String,
    authority_host: AzureAuthorityHost,
}

impl AzureKeyVaultProvider {
    pub fn new(
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        tenant_id: impl Into<String>,
        vault_url: impl Into<String>,
        authority_host: impl Into<AzureAuthorityHost>,
    ) -> Self {
        Self {
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            tenant_id: tenant_id.into(),
            vault_url: vault_url.into(),
            authority_host: authority_host.into(),
        }
    }
}

#[async_trait]
impl Provider for AzureKeyVaultProvider {
    #[instrument(name = "spin_variables.get_from_azure_key_vault", skip(self), err(level = Level::INFO), fields(otel.kind = "client"))]
    async fn get(&self, key: &Key) -> Result<Option<String>> {
        let http_client = azure_core::new_http_client();
        let credential = azure_identity::ClientSecretCredential::new(
            http_client,
            self.authority_host.into(),
            self.tenant_id.to_string(),
            self.client_id.to_string(),
            self.client_secret.to_string(),
        );

        let secret_client = SecretClient::new(&self.vault_url, Arc::new(credential))?;
        match secret_client.get(key.as_str()).await {
            Ok(secret) => Ok(Some(secret.value)),
            Err(err) => return Err(err).context("Failed to read variable from Azure Key Vault"),
        }
    }
}

#[derive(Debug, Copy, Clone, Deserialize)]
pub enum AzureAuthorityHost {
    AzurePublicCloud,
    AzureChina,
    AzureGermany,
    AzureGovernment,
}

impl Default for AzureAuthorityHost {
    fn default() -> Self {
        Self::AzurePublicCloud
    }
}

impl From<AzureAuthorityHost> for Url {
    fn from(value: AzureAuthorityHost) -> Self {
        let url = match value {
            AzureAuthorityHost::AzureChina => "https://login.chinacloudapi.cn/",
            AzureAuthorityHost::AzureGovernment => "https://login.microsoftonline.us/",
            AzureAuthorityHost::AzureGermany => "https://login.microsoftonline.de/",
            _ => "https://login.microsoftonline.com/",
        };
        Url::parse(url).unwrap()
    }
}
impl From<&str> for AzureAuthorityHost {
    fn from(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "azurechina" | "china" => AzureAuthorityHost::AzureChina,
            "azuregermany" | "germany" => AzureAuthorityHost::AzureGermany,
            "AzureGovernment" | "gov" => AzureAuthorityHost::AzureGovernment,
            _ => AzureAuthorityHost::AzurePublicCloud,
        }
    }
}

impl From<String> for AzureAuthorityHost {
    fn from(value: String) -> Self {
        AzureAuthorityHost::from(value.as_str())
    }
}
