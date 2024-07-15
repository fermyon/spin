use std::{collections::HashMap, sync::Arc};

use serde::Deserialize;
use spin_factors::{anyhow, FactorRuntimeConfig};

use crate::ConnectionPool;

#[derive(Deserialize)]
#[serde(transparent)]
pub struct RuntimeConfig {
    pub store_configs: HashMap<String, StoreConfig>,
}

impl FactorRuntimeConfig for RuntimeConfig {
    const KEY: &'static str = "sqlite_database";
}

#[derive(Deserialize)]
pub struct StoreConfig {
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(flatten)]
    pub config: toml::Table,
}

/// Resolves some piece of runtime configuration to a connection pool
pub trait RuntimeConfigResolver {
    /// Get a connection pool for a given runtime configuration type and the raw configuration.
    fn get_pool(
        &self,
        r#type: &str,
        config: toml::Table,
    ) -> anyhow::Result<Arc<dyn ConnectionPool>>;

    /// If there is no runtime configuration for a given database label, return a default connection pool.
    ///
    /// If `Option::None` is returned, the database is not allowed.
    fn default(&self, label: &str) -> Option<Arc<dyn ConnectionPool>>;
}
