use std::sync::Arc;

use anyhow::Context;
use serde::de::DeserializeOwned;
use spin_key_value::StoreManager;

pub trait MakeKeyValueStore: 'static + Send + Sync {
    const RUNTIME_CONFIG_TYPE: &'static str;

    type RuntimeConfig: DeserializeOwned;
    type StoreManager: StoreManager;

    fn make_store(&self, runtime_config: Self::RuntimeConfig)
        -> anyhow::Result<Self::StoreManager>;
}

pub(crate) type StoreFromToml =
    Box<dyn Fn(toml::Table) -> anyhow::Result<Arc<dyn StoreManager>> + Send + Sync>;

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
