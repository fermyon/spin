use std::{str::FromStr, sync::Arc};

use anyhow::{bail, Context, Result};
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
        let secret = secret_client
            .get(key.as_str())
            .await
            .context("Failed to read variable from Azure Key Vault")?;
        Ok(Some(secret.value))
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
            AzureAuthorityHost::AzurePublicCloud => "https://login.microsoftonline.com/",
        };
        Url::parse(url).unwrap()
    }
}
impl FromStr for AzureAuthorityHost {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::prelude::v1::Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "azurechina" | "china" => AzureAuthorityHost::AzureChina,
            "azuregermany" | "germany" => AzureAuthorityHost::AzureGermany,
            "azuregovernment" | "gov" => AzureAuthorityHost::AzureGovernment,
            "azureublic" | "azurepubliccloud" | "public" | "publiccloud" => {
                AzureAuthorityHost::AzurePublicCloud
            }
            _ => bail!("Unsupported value provided for AzureAuthorityHost"),
        })
    }
}

impl From<String> for AzureAuthorityHost {
    fn from(value: String) -> Self {
        value.as_str().parse().unwrap()
    }
}
