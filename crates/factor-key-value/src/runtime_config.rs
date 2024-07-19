use std::{collections::HashMap, sync::Arc};

use serde::{de::DeserializeOwned, Deserialize};
use spin_factors::{anyhow, FactorRuntimeConfig};
use spin_key_value::StoreManager;

/// Runtime configuration for all key value stores.
#[derive(Deserialize)]
#[serde(transparent)]
pub struct RuntimeConfig<C> {
    /// Map of store names to store configurations.
    pub store_configs: HashMap<String, C>,
}

impl<C: DeserializeOwned> FactorRuntimeConfig for RuntimeConfig<C> {
    const KEY: &'static str = "key_value_store";
}

/// Resolves some piece of runtime configuration to a key value store manager.
pub trait RuntimeConfigResolver: Send + Sync {
    /// The type of configuration that this resolver can handle.
    type Config: DeserializeOwned;

    /// Get a store manager for a given config.
    fn get_store(&self, config: Self::Config) -> anyhow::Result<Arc<dyn StoreManager>>;

    /// Returns a default store manager for a given label. Should only be called
    /// if there is no runtime configuration for the label.
    ///
    /// If `Option::None` is returned, the database is not allowed.
    fn default_store(&self, label: &str) -> Option<Arc<dyn StoreManager>>;
}
