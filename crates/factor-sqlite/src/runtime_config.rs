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
pub trait RuntimeConfigResolver: Send + Sync {
    /// Get a connection pool for a given database kind and the raw configuration.
    ///
    /// `database_kind` is equivalent to the `type` field in the
    /// `[sqlite_database.$databasename]` runtime configuration table.
    fn get_pool(
        &self,
        database_kind: &str,
        config: toml::Table,
    ) -> anyhow::Result<Arc<dyn ConnectionPool>>;

    /// If there is no runtime configuration for a given database label, return a default connection pool.
    ///
    /// If `Option::None` is returned, the database is not allowed.
    fn default(&self, label: &str) -> Option<Arc<dyn ConnectionPool>>;
}
