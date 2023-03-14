use anyhow::Result;
use std::{collections::HashSet, sync::Arc};
use table::Table;
use wit_bindgen_wasmtime::async_trait;

mod host_component;
mod table;
mod util;

pub use host_component::{component_key_value_stores, manager, KeyValueComponent};
pub use util::{CachingStoreManager, DelegatingStoreManager, EmptyStoreManager};

const DEFAULT_STORE_TABLE_CAPACITY: u32 = 256;

wit_bindgen_wasmtime::export!({paths: ["../../wit/ephemeral/key-value.wit"], async: *});

pub use key_value::{Error, KeyValue, Store as StoreHandle};

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error {}

#[async_trait]
pub trait StoreManager: Sync + Send {
    async fn get(&self, name: &str) -> Result<Arc<dyn Store>, Error>;
}

#[async_trait]
pub trait Store: Sync + Send {
    async fn get(&self, key: &str) -> Result<Vec<u8>, Error>;

    async fn set(&self, key: &str, value: &[u8]) -> Result<(), Error>;

    async fn delete(&self, key: &str) -> Result<(), Error>;

    async fn exists(&self, key: &str) -> Result<bool, Error>;

    async fn get_keys(&self) -> Result<Vec<String>, Error>;
}

pub struct KeyValueDispatch {
    allowed_stores: HashSet<String>,
    manager: Arc<dyn StoreManager>,
    stores: Table<Arc<dyn Store>>,
}

impl KeyValueDispatch {
    pub fn new() -> Self {
        Self::new_with_capacity(DEFAULT_STORE_TABLE_CAPACITY)
    }

    pub fn new_with_capacity(capacity: u32) -> Self {
        Self {
            allowed_stores: HashSet::new(),
            manager: Arc::new(EmptyStoreManager),
            stores: Table::new(capacity),
        }
    }

    pub fn init(&mut self, allowed_stores: HashSet<String>, manager: Arc<dyn StoreManager>) {
        self.allowed_stores = allowed_stores;
        self.manager = manager;
    }
}

impl Default for KeyValueDispatch {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl KeyValue for KeyValueDispatch {
    async fn open(&mut self, name: &str) -> Result<StoreHandle, Error> {
        if self.allowed_stores.contains(name) {
            self.stores
                .push(self.manager.get(name).await?)
                .map_err(|()| Error::StoreTableFull)
        } else {
            Err(Error::AccessDenied)
        }
    }

    async fn get(&mut self, store: StoreHandle, key: &str) -> Result<Vec<u8>, Error> {
        self.stores
            .get(store)
            .ok_or(Error::InvalidStore)?
            .get(key)
            .await
    }

    async fn set(&mut self, store: StoreHandle, key: &str, value: &[u8]) -> Result<(), Error> {
        self.stores
            .get(store)
            .ok_or(Error::InvalidStore)?
            .set(key, value)
            .await
    }

    async fn delete(&mut self, store: StoreHandle, key: &str) -> Result<(), Error> {
        self.stores
            .get(store)
            .ok_or(Error::InvalidStore)?
            .delete(key)
            .await
    }

    async fn exists(&mut self, store: StoreHandle, key: &str) -> Result<bool, Error> {
        self.stores
            .get(store)
            .ok_or(Error::InvalidStore)?
            .exists(key)
            .await
    }

    async fn get_keys(&mut self, store: StoreHandle) -> Result<Vec<String>, Error> {
        self.stores
            .get(store)
            .ok_or(Error::InvalidStore)?
            .get_keys()
            .await
    }

    async fn close(&mut self, store: StoreHandle) {
        self.stores.remove(store);
    }
}

pub fn log_error(err: impl std::fmt::Debug) -> Error {
    tracing::warn!("key-value error: {err:?}");
    Error::Io(format!("{err:?}"))
}
