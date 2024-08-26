pub mod spin;

use std::{collections::HashMap, sync::Arc};

use spin_key_value::StoreManager;

/// Runtime configuration for all key value stores.
#[derive(Default, Clone)]
pub struct RuntimeConfig {
    /// Map of store names to store managers.
    store_managers: HashMap<String, Arc<dyn StoreManager>>,
}

impl RuntimeConfig {
    /// Adds a store manager for the store with the given label to the runtime configuration.
    ///
    /// If a store manager already exists for the given label, it will be replaced.
    pub fn add_store_manager(&mut self, label: String, store_manager: Arc<dyn StoreManager>) {
        self.store_managers.insert(label, store_manager);
    }
}

impl IntoIterator for RuntimeConfig {
    type Item = (String, Arc<dyn StoreManager>);
    type IntoIter = std::collections::hash_map::IntoIter<String, Arc<dyn StoreManager>>;

    fn into_iter(self) -> Self::IntoIter {
        self.store_managers.into_iter()
    }
}
