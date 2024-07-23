//! The runtime configuration for the variables factor used in the Spin CLI.

mod env;
mod statik;

pub use env::EnvVariables;
pub use statik::StaticVariables;

use serde::Deserialize;
use statik::StaticVariablesProvider;

/// The runtime configuration for the variables factor used in the Spin CLI.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum RuntimeConfig {
    /// A static provider of variables.
    Static(StaticVariablesProvider),
    /// An environment variable provider.
    Env(env::EnvVariablesConfig),
}
