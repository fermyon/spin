use crate::{Error, Store, StoreManager};
use spin_core::async_trait;
use std::{collections::HashMap, sync::Arc};

/// A [`StoreManager`] which delegates to other `StoreManager`s based on the store label.
pub struct DelegatingStoreManager {
    delegates: HashMap<String, Arc<dyn StoreManager>>,
}

impl DelegatingStoreManager {
    pub fn new(delegates: impl IntoIterator<Item = (String, Arc<dyn StoreManager>)>) -> Self {
        let delegates = delegates.into_iter().collect();
        Self { delegates }
    }
}

#[async_trait]
impl StoreManager for DelegatingStoreManager {
    async fn get(&self, name: &str) -> Result<Arc<dyn Store>, Error> {
        match self.delegates.get(name) {
            Some(store) => store.get(name).await,
            None => Err(Error::NoSuchStore),
        }
    }

    fn is_defined(&self, store_name: &str) -> bool {
        self.delegates.contains_key(store_name)
    }

    fn summary(&self, store_name: &str) -> Option<String> {
        if let Some(store) = self.delegates.get(store_name) {
            return store.summary(store_name);
        }
        None
    }
}
