use std::sync::Arc;

use anyhow::Context;
use serde::de::DeserializeOwned;
use spin_key_value::StoreManager;

/// Defines the construction of a key value store.
pub trait MakeKeyValueStore: 'static + Send + Sync {
    /// Unique type identifier for the store.
    const RUNTIME_CONFIG_TYPE: &'static str;
    /// Runtime configuration for the store.
    type RuntimeConfig: DeserializeOwned;
    /// The store manager for the store.
    type StoreManager: StoreManager;

    /// Creates a new store manager from the runtime configuration.
    fn make_store(&self, runtime_config: Self::RuntimeConfig)
        -> anyhow::Result<Self::StoreManager>;
}

/// A function that creates a store manager from a TOML table.
pub(crate) type StoreFromToml =
    Box<dyn Fn(toml::Table) -> anyhow::Result<Arc<dyn StoreManager>> + Send + Sync>;

/// Creates a `StoreFromToml` function from a `MakeKeyValueStore` implementation.
pub(crate) fn store_from_toml_fn<T: MakeKeyValueStore>(provider_type: T) -> StoreFromToml {
    Box::new(move |table| {
        let runtime_config: T::RuntimeConfig =
            table.try_into().context("could not parse runtime config")?;
        let provider = provider_type
            .make_store(runtime_config)
            .context("could not make store")?;
        Ok(Arc::new(provider))
    })
}
