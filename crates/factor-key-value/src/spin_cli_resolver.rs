use crate::runtime_config::RuntimeConfigResolver;
use crate::store::{store_from_toml_fn, MakeKeyValueStore, StoreFromToml};
use spin_key_value::StoreManager;
use std::{collections::HashMap, sync::Arc};

#[derive(Default)]
pub struct SpinCliRuntimeConfigResolver {
    store_types: HashMap<&'static str, StoreFromToml>,
}

impl SpinCliRuntimeConfigResolver {
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

impl RuntimeConfigResolver for SpinCliRuntimeConfigResolver {
    fn get_store(
        &self,
        store_kind: &str,
        config: toml::Table,
    ) -> anyhow::Result<Arc<dyn StoreManager>> {
        let store_from_toml = self
            .store_types
            .get(store_kind)
            .ok_or_else(|| anyhow::anyhow!("unknown store kind: {}", store_kind))?;
        store_from_toml(config)
    }

    fn default(&self, label: &str) -> Option<Arc<dyn StoreManager>> {
        if label == "default" {
            self.get_store("spin", toml::value::Table::new()).ok()
        } else {
            None
        }
    }
}
