use crate::{Error, Store, StoreManager};
use std::{collections::HashMap, sync::Arc};
use wit_bindgen_wasmtime::async_trait;

pub struct EmptyStoreManager;

#[async_trait]
impl StoreManager for EmptyStoreManager {
    async fn get(&self, _name: &str) -> Result<Arc<dyn Store>, Error> {
        Err(Error::NoSuchStore)
    }
}

pub struct DelegatingStoreManager {
    delegates: HashMap<String, Arc<dyn StoreManager>>,
}

impl DelegatingStoreManager {
    pub fn new(delegates: HashMap<String, Arc<dyn StoreManager>>) -> Self {
        Self { delegates }
    }
}

#[async_trait]
impl StoreManager for DelegatingStoreManager {
    async fn get(&self, name: &str) -> Result<Arc<dyn Store>, Error> {
        self.delegates
            .get(name)
            .ok_or(Error::NoSuchStore)?
            .get(name)
            .await
    }
}
