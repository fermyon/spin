use std::path::PathBuf;

use anyhow::{anyhow, Result};
use serde::Deserialize;
use spin_variables::provider::azure_key_vault::{
    AzureKeyVaultAuthOptions, AzureKeyVaultRuntimeConfigOptions,
};
use spin_variables::provider::{
    azure_key_vault::{AzureAuthorityHost, AzureKeyVaultProvider},
    env::EnvProvider,
    vault::VaultProvider,
};

use super::RuntimeConfig;

pub type VariablesProvider = Box<dyn spin_expressions::Provider>;

// Holds deserialized options from a `[[config_provider]]` runtime config section.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum VariablesProviderOpts {
    Env(EnvVariablesProviderOpts),
    Vault(VaultVariablesProviderOpts),
    AzureKeyVault(AzureKeyVaultVariablesProviderOpts),
}

impl VariablesProviderOpts {
    pub fn default_provider_opts(runtime_config: &RuntimeConfig) -> Self {
        Self::Env(EnvVariablesProviderOpts::default_provider_opts(
            runtime_config,
        ))
    }

    pub fn build_provider(&self) -> Result<VariablesProvider> {
        match self {
            Self::Env(opts) => opts.build_provider(),
            Self::Vault(opts) => opts.build_provider(),
            Self::AzureKeyVault(opts) => opts.build_provider(),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvVariablesProviderOpts {
    /// A prefix to add to variable names when resolving from the environment.
    /// Unless empty, joined to the variable name with an underscore.
    #[serde(default)]
    pub prefix: Option<String>,
    /// Optional path to a 'dotenv' file which will be merged into the environment.
    #[serde(default)]
    pub dotenv_path: Option<PathBuf>,
}

impl EnvVariablesProviderOpts {
    pub fn default_provider_opts(runtime_config: &RuntimeConfig) -> Self {
        let dotenv_path = runtime_config
            .local_app_dir
            .as_deref()
            .map(|path| path.join(".env"));
        Self {
            prefix: None,
            dotenv_path,
        }
    }

    pub fn build_provider(&self) -> Result<VariablesProvider> {
        Ok(Box::new(EnvProvider::new(
            self.prefix.clone(),
            self.dotenv_path.clone(),
        )))
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VaultVariablesProviderOpts {
    pub url: String,
    pub token: String,
    pub mount: String,
    #[serde(default)]
    pub prefix: Option<String>,
}

impl VaultVariablesProviderOpts {
    pub fn build_provider(&self) -> Result<VariablesProvider> {
        Ok(Box::new(VaultProvider::new(
            &self.url,
            &self.token,
            &self.mount,
            self.prefix.as_deref(),
        )))
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AzureKeyVaultVariablesProviderOpts {
    pub vault_url: String,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub tenant_id: Option<String>,
    pub authority_host: Option<AzureAuthorityHost>,
}

impl AzureKeyVaultVariablesProviderOpts {
    pub fn build_provider(&self) -> Result<VariablesProvider> {
        let auth_config_runtime_vars = [&self.client_id, &self.tenant_id, &self.client_secret];
        let any_some = auth_config_runtime_vars.iter().any(|&var| var.is_some());
        let any_none = auth_config_runtime_vars.iter().any(|&var| var.is_none());

        if any_none && any_some {
            // some of the service principal auth options were specified, but not enough to authenticate.
            return Err(anyhow!("The current runtime config specifies some but not all of the Azure KeyVault 'client_id', 'client_secret', and 'tenant_id' values. Provide the missing values to authenticate to Azure KeyVault with the given service principal, or remove all these values to authenticate using ambient authentication (e.g. env vars, Azure CLI, Managed Identity, Workload Identity)."));
        }

        let auth_options = if any_some {
            // all the service principal auth options were specified in the runtime config
            AzureKeyVaultAuthOptions::RuntimeConfigValues(AzureKeyVaultRuntimeConfigOptions::new(
                self.client_id.clone().unwrap(),
                self.client_secret.clone().unwrap(),
                self.tenant_id.clone().unwrap(),
                self.authority_host,
            ))
        } else {
            AzureKeyVaultAuthOptions::Environmental
        };

        Ok(Box::new(AzureKeyVaultProvider::new(
            &self.vault_url,
            auth_options,
        )?))
    }
}
