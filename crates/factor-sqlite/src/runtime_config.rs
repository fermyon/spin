#[cfg(feature = "spin-cli")]
pub mod spin;

use std::{collections::HashMap, sync::Arc};

use serde::{de::DeserializeOwned, Deserialize};
use spin_factors::{anyhow, FactorRuntimeConfig};

use crate::ConnectionPool;

#[derive(Deserialize)]
#[serde(transparent)]
pub struct RuntimeConfig<C> {
    pub store_configs: HashMap<String, C>,
}

impl<C: DeserializeOwned> FactorRuntimeConfig for RuntimeConfig<C> {
    const KEY: &'static str = "sqlite_database";
}

/// Resolves some piece of runtime configuration to a connection pool
pub trait RuntimeConfigResolver: Send + Sync {
    type Config;

    /// Get a connection pool for a given config.
    ///
    fn get_pool(&self, config: Self::Config) -> anyhow::Result<Arc<dyn ConnectionPool>>;

    /// If there is no runtime configuration for a given database label, return a default connection pool.
    ///
    /// If `Option::None` is returned, the database is not allowed.
    fn default(&self, label: &str) -> Option<Arc<dyn ConnectionPool>>;
}
