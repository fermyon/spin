use std::sync::Arc;

use serde::de::DeserializeOwned;
use spin_key_value::StoreManager;

pub trait MakeKeyValueStore: 'static {
    const RUNTIME_CONFIG_TYPE: &'static str;

    type RuntimeConfig: DeserializeOwned;
    type StoreManager: StoreManager;

    fn make_store(&self, runtime_config: Self::RuntimeConfig)
        -> anyhow::Result<Self::StoreManager>;
}

pub(crate) type StoreFromToml = Box<dyn Fn(toml::Table) -> anyhow::Result<Arc<dyn StoreManager>>>;

pub(crate) fn store_from_toml_fn<T: MakeKeyValueStore>(provider_type: T) -> StoreFromToml {
    Box::new(move |table| {
        let runtime_config: T::RuntimeConfig = table.try_into()?;
        let provider = provider_type.make_store(runtime_config)?;
        Ok(Arc::new(provider))
    })
}
