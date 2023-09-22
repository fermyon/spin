use std::path::PathBuf;

use serde::Deserialize;
use spin_config::provider::{env::EnvProvider, vault::VaultProvider, azkv::AzureKeyVaultProvider};

use super::RuntimeConfig;

pub type ConfigProvider = Box<dyn spin_config::Provider>;

const DEFAULT_ENV_PREFIX: &str = "SPIN_CONFIG";

// Holds deserialized options from a `[[config_provider]]` runtime config section.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ConfigProviderOpts {
    Env(EnvConfigProviderOpts),
    Vault(VaultConfigProviderOpts),
    AzureKeyVault(AzureKeyVaultConfigProviderOpts),
}

impl ConfigProviderOpts {
    pub fn default_provider_opts(runtime_config: &RuntimeConfig) -> Self {
        Self::Env(EnvConfigProviderOpts::default_provider_opts(runtime_config))
    }

    pub fn build_provider(&self) -> ConfigProvider {
        match self {
            Self::Env(opts) => opts.build_provider(),
            Self::Vault(opts) => opts.build_provider(),
            Self::AzureKeyVault(opts) => opts.build_provider(),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvConfigProviderOpts {
    /// A prefix to add to variable names when resolving from the environment.
    /// Unless empty, joined to the variable name with an underscore.
    pub prefix: String,
    /// Optional path to a 'dotenv' file which will be merged into the environment.
    #[serde(default)]
    pub dotenv_path: Option<PathBuf>,
}

impl EnvConfigProviderOpts {
    pub fn default_provider_opts(runtime_config: &RuntimeConfig) -> Self {
        let dotenv_path = runtime_config
            .local_app_dir
            .as_deref()
            .map(|path| path.join(".env"));
        Self {
            prefix: DEFAULT_ENV_PREFIX.to_string(),
            dotenv_path,
        }
    }

    pub fn build_provider(&self) -> ConfigProvider {
        Box::new(EnvProvider::new(&self.prefix, self.dotenv_path.clone()))
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VaultConfigProviderOpts {
    pub url: String,
    pub token: String,
    pub mount: String,
    #[serde(default)]
    pub prefix: Option<String>,
}

impl VaultConfigProviderOpts {
    pub fn build_provider(&self) -> ConfigProvider {
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
pub struct AzureKeyVaultConfigProviderOpts {
    pub client_id: String,
    pub client_secret: String,
    pub tenant_id: String,
    pub url: String,
}

impl AzureKeyVaultConfigProviderOpts {
    pub fn build_provider(&self) -> ConfigProvider {
        Box::new(AzureKeyVaultProvider::new(
            &self.client_id,
            &self.client_secret,
            &self.tenant_id,
            &self.url,
        ))
    }
}
