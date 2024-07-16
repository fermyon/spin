use std::{collections::HashMap, sync::Arc};

use serde::Deserialize;
use spin_factors::{anyhow, FactorRuntimeConfig};
use spin_key_value::StoreManager;

#[derive(Deserialize)]
#[serde(transparent)]
pub struct RuntimeConfig {
    pub store_configs: HashMap<String, StoreConfig>,
}

impl FactorRuntimeConfig for RuntimeConfig {
    const KEY: &'static str = "key_value_store";
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
    /// Get a store manager for a given store kind and the raw configuration.
    ///
    /// `store_kind` is equivalent to the `type` field in the
    /// `[key_value_store.$storename]` runtime configuration table.
    fn get_store(
        &self,
        store_kind: &str,
        config: toml::Table,
    ) -> anyhow::Result<Arc<dyn StoreManager>>;

    /// Returns a default store manager for a given label. Should only be called
    /// if there is no runtime configuration for the label.
    ///
    /// If `Option::None` is returned, the database is not allowed.
    fn default(&self, label: &str) -> Option<Arc<dyn StoreManager>>;
}
