use std::path::PathBuf;

use serde::Deserialize;
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

    pub fn build_provider(&self) -> VariablesProvider {
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

    pub fn build_provider(&self) -> VariablesProvider {
        Box::new(EnvProvider::new(
            self.prefix.clone(),
            self.dotenv_path.clone(),
        ))
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
    pub fn build_provider(&self) -> VariablesProvider {
        Box::new(VaultProvider::new(
            &self.url,
            &self.token,
            &self.mount,
            self.prefix.as_deref(),
        ))
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AzureKeyVaultVariablesProviderOpts {
    pub client_id: String,
    pub client_secret: String,
    pub tenant_id: String,
    pub vault_url: String,
    #[serde(default)]
    pub authority_host: AzureAuthorityHost,
}

impl AzureKeyVaultVariablesProviderOpts {
    pub fn build_provider(&self) -> VariablesProvider {
        Box::new(AzureKeyVaultProvider::new(
            &self.client_id,
            &self.client_secret,
            &self.tenant_id,
            &self.vault_url,
            self.authority_host,
        ))
    }
}
