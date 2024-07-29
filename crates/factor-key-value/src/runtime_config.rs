use std::{collections::HashMap, sync::Arc};

use spin_key_value::StoreManager;

/// Runtime configuration for all key value stores.
#[derive(Default)]
pub struct RuntimeConfig {
    /// Map of store names to store managers.
    pub store_managers: HashMap<String, Arc<dyn StoreManager>>,
}

impl IntoIterator for RuntimeConfig {
    type Item = (String, Arc<dyn StoreManager>);
    type IntoIter = std::collections::hash_map::IntoIter<String, Arc<dyn StoreManager>>;

    fn into_iter(self) -> Self::IntoIter {
        self.store_managers.into_iter()
    }
}
