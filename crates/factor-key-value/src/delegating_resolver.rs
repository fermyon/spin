use crate::{
    store::{store_from_toml_fn, MakeKeyValueStore, StoreFromToml},
    DefaultLabelResolver, RuntimeConfig,
};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use spin_key_value::StoreManager;
use std::{collections::HashMap, sync::Arc};

/// Converts from toml based runtime configuration into a [`RuntimeConfig`].
///
/// Also acts as [`DefaultLabelResolver`].
///
/// The various store types (i.e., the "type" field in the toml field) are registered with the
/// resolver using `add_store_type`. The default store for a label is registered using `add_default_store`.
#[derive(Default)]
pub struct RuntimeConfigResolver {
    /// A map of store types to a function that returns the appropriate store
    /// manager from runtime config TOML.
    store_types: HashMap<&'static str, StoreFromToml>,
    /// A map of default store configurations for a label.
    defaults: HashMap<&'static str, StoreConfig>,
}

impl RuntimeConfigResolver {
    /// Create a new RuntimeConfigResolver.
    pub fn new() -> Self {
        <Self as Default>::default()
    }

    /// Adds a default store configuration for a label.
    ///
    /// Users must ensure that the store type for `config` has been registered with
    /// the resolver using [`Self::register_store_type`].
    pub fn add_default_store(&mut self, label: &'static str, config: StoreConfig) {
        self.defaults.insert(label, config);
    }

    /// Registers a store type to the resolver.
    pub fn register_store_type<T: MakeKeyValueStore>(
        &mut self,
        store_type: T,
    ) -> anyhow::Result<()> {
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

    /// Resolves a toml table into a runtime config.
    pub fn resolve_from_toml(
        &self,
        table: &Option<toml::Table>,
    ) -> anyhow::Result<Option<RuntimeConfig>> {
        let Some(table) = table.as_ref().and_then(|t| t.get("key_value_store")) else {
            return Ok(None);
        };
        let mut store_configs = HashMap::new();
        for (label, config) in table
            .as_table()
            .context("expected a 'key_value_store' to contain toml table")?
        {
            let config: StoreConfig = config.clone().try_into()?;
            let store_manager = self.store_manager_from_config(config)?;
            store_configs.insert(label.clone(), store_manager);
        }
        Ok(Some(RuntimeConfig {
            store_managers: store_configs,
        }))
    }

    /// Given a [`StoreConfig`], returns a store manager.
    ///
    /// Errors if there is no [`MakeKeyValueStore`] registered for the store config's type
    /// or if the store manager cannot be created from the config.
    fn store_manager_from_config(
        &self,
        config: StoreConfig,
    ) -> anyhow::Result<Arc<dyn StoreManager>> {
        let config_type = config.type_.as_str();
        let maker = self.store_types.get(config_type).with_context(|| {
            format!("the store type '{config_type}' was not registered with the config resolver")
        })?;
        maker(config.config)
    }
}

impl DefaultLabelResolver for RuntimeConfigResolver {
    fn default(&self, label: &str) -> Option<Arc<dyn StoreManager>> {
        let config = self.defaults.get(label)?;
        Some(self.store_manager_from_config(config.clone()).unwrap())
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
