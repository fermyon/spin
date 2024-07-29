//! The runtime configuration for the variables factor used in the Spin CLI.

mod env;
mod statik;

use serde::Deserialize;
use spin_expressions::Provider;
use spin_factors::anyhow;
use statik::StaticVariablesProvider;

use crate::runtime_config::RuntimeConfig;

/// A runtime configuration used in the Spin CLI for one type of variable provider.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum VariableProviderConfiguration {
    /// A static provider of variables.
    Static(StaticVariablesProvider),
    /// An environment variable provider.
    Env(env::EnvVariablesConfig),
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
        }
    }
}

/// Resolves a runtime configuration for the variables factor from a TOML table.
pub fn runtime_config_from_toml(table: &toml::Table) -> anyhow::Result<Option<RuntimeConfig>> {
    let Some(array) = table.get("variable_provider") else {
        return Ok(None);
    };

    let provider_configs: Vec<VariableProviderConfiguration> = array.clone().try_into()?;
    let providers = provider_configs
        .into_iter()
        .map(VariableProviderConfiguration::into_provider)
        .collect();
    Ok(Some(RuntimeConfig { providers }))
}
