use crate::runtime_config::RuntimeConfigResolver;
use crate::store::{store_from_toml_fn, MakeKeyValueStore, StoreFromToml};
use serde::{Deserialize, Serialize};
use spin_key_value::StoreManager;
use std::{collections::HashMap, sync::Arc};

/// A RuntimeConfigResolver that delegates to the appropriate key-value store
/// for a given label.
///
/// The store types are registered with the resolver using `add_store_type`. The
/// default store for a label is registered using `add_default_store`.
#[derive(Default)]
pub struct DelegatingRuntimeConfigResolver {
    /// A map of store types to a function that returns the appropriate store
    /// manager from runtime config TOML.
    store_types: HashMap<&'static str, StoreFromToml>,
    /// A map of default store configurations for a label.
    defaults: HashMap<&'static str, StoreConfig>,
}

impl DelegatingRuntimeConfigResolver {
    /// Create a new DelegatingRuntimeConfigResolver.
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_default_store(&mut self, label: &'static str, config: StoreConfig) {
        self.defaults.insert(label, config);
    }

    /// Adds a store type to the resolver.
    pub fn add_store_type<T: MakeKeyValueStore>(&mut self, store_type: T) -> anyhow::Result<()> {
        if self
            .store_types
            .insert(T::RUNTIME_CONFIG_TYPE, store_from_toml_fn(store_type))
            .is_some()
        {
            anyhow::bail!(
                "duplicate key value store type {:?}",
                T::RUNTIME_CONFIG_TYPE
            );
        }
        Ok(())
    }
}

impl RuntimeConfigResolver for DelegatingRuntimeConfigResolver {
    type Config = StoreConfig;

    fn get_store(&self, config: StoreConfig) -> anyhow::Result<Arc<dyn StoreManager>> {
        let store_kind = config.type_.as_str();
        let store_from_toml = self
            .store_types
            .get(store_kind)
            .ok_or_else(|| anyhow::anyhow!("unknown store kind: {}", store_kind))?;
        store_from_toml(config.config)
    }

    /// Get the default store manager for the given label.
    ///
    /// Returns None if no default store is registered for the label.
    fn default_store(&self, label: &str) -> Option<Arc<dyn StoreManager>> {
        let config = self.defaults.get(label)?;
        self.get_store(config.clone()).ok()
    }
}

#[derive(Deserialize, Clone)]
pub struct StoreConfig {
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(flatten)]
    pub config: toml::Table,
}

impl StoreConfig {
    pub fn new<T>(type_: String, config: T) -> anyhow::Result<Self>
    where
        T: Serialize,
    {
        Ok(Self {
            type_,
            config: toml::value::Table::try_from(config)?,
        })
    }
}
