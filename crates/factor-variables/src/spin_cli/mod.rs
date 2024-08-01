//! The runtime configuration for the variables factor used in the Spin CLI.

mod env;
mod statik;
mod vault;

pub use env::*;
pub use statik::*;
pub use vault::*;

use serde::Deserialize;
use spin_expressions::Provider;
use spin_factors::anyhow;

use crate::runtime_config::RuntimeConfig;

/// Resolves a runtime configuration for the variables factor from a TOML table.
pub fn runtime_config_from_toml(table: &toml::Table) -> anyhow::Result<RuntimeConfig> {
    // Always include the environment variable provider.
    let mut providers = vec![Box::<EnvVariablesProvider>::default() as _];
    let Some(array) = table.get("variable_provider") else {
        return Ok(RuntimeConfig { providers });
    };

    let provider_configs: Vec<VariableProviderConfiguration> = array.clone().try_into()?;
    providers.extend(
        provider_configs
            .into_iter()
            .map(VariableProviderConfiguration::into_provider),
    );
    Ok(RuntimeConfig { providers })
}

/// A runtime configuration used in the Spin CLI for one type of variable provider.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum VariableProviderConfiguration {
    /// A static provider of variables.
    Static(StaticVariablesProvider),
    /// A provider that uses HashiCorp Vault.
    Vault(VaultVariablesProvider),
    /// An environment variable provider.
    Env(EnvVariablesConfig),
}

impl VariableProviderConfiguration {
    /// Returns the provider for the configuration.
    pub fn into_provider(self) -> Box<dyn Provider> {
        match self {
            VariableProviderConfiguration::Static(provider) => Box::new(provider),
            VariableProviderConfiguration::Env(config) => Box::new(env::EnvVariablesProvider::new(
                config.prefix,
                |s| std::env::var(s),
                config.dotenv_path,
            )),
            VariableProviderConfiguration::Vault(provider) => Box::new(provider),
        }
    }
}
