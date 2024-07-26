//! The runtime configuration for the variables factor used in the Spin CLI.

mod env;
mod statik;

pub use env::EnvVariables;
pub use statik::StaticVariables;

use serde::Deserialize;
use statik::StaticVariablesProvider;

/// A runtime configuration used in the Spin CLI for one type of variable provider.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum VariableProviderConfiguration {
    /// A static provider of variables.
    Static(StaticVariablesProvider),
    /// An environment variable provider.
    Env(env::EnvVariablesConfig),
}

/// The runtime configuration for the variables factor used in the Spin CLI.
pub type RuntimeConfig = super::RuntimeConfig<VariableProviderConfiguration>;

/// The variables factor used in the Spin CLI.
pub type VariablesFactor = super::VariablesFactor<VariableProviderConfiguration>;
