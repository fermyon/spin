//! The runtime configuration for the variables factor used in the Spin CLI.

mod env;
mod statik;

pub use env::EnvVariables;
pub use statik::StaticVariables;

use serde::Deserialize;
use statik::StaticVariablesProvider;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum RuntimeConfig {
    Static(StaticVariablesProvider),
    Env(env::EnvVariablesConfig),
}
