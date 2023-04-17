use anyhow::Result;
use spin_app::MetadataKey;
use spin_core::{async_trait, key_value};
use std::{collections::HashSet, sync::Arc};
use table::Table;

mod host_component;
mod table;
mod util;

pub use host_component::{manager, KeyValueComponent};
pub use util::{CachingStoreManager, DelegatingStoreManager, EmptyStoreManager};

pub const KEY_VALUE_STORES_KEY: MetadataKey<Vec<String>> = MetadataKey::new("key_value_stores");

const DEFAULT_STORE_TABLE_CAPACITY: u32 = 256;

pub use key_value::{Error, Store as StoreHandle};

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
impl key_value::Host for KeyValueDispatch {
    async fn open(&mut self, name: String) -> Result<Result<StoreHandle, Error>> {
        Ok(async {
            if self.allowed_stores.contains(&name) {
                self.stores
                    .push(self.manager.get(&name).await?)
                    .map_err(|()| Error::StoreTableFull)
            } else {
                Err(Error::AccessDenied)
            }
        }
        .await)
    }

    async fn get(&mut self, store: StoreHandle, key: String) -> Result<Result<Vec<u8>, Error>> {
        Ok(async {
            self.stores
                .get(store)
                .ok_or(Error::InvalidStore)?
                .get(&key)
                .await
        }
        .await)
    }

    async fn set(
        &mut self,
        store: StoreHandle,
        key: String,
        value: Vec<u8>,
    ) -> Result<Result<(), Error>> {
        Ok(async {
            self.stores
                .get(store)
                .ok_or(Error::InvalidStore)?
                .set(&key, &value)
                .await
        }
        .await)
    }

    async fn delete(&mut self, store: StoreHandle, key: String) -> Result<Result<(), Error>> {
        Ok(async {
            self.stores
                .get(store)
                .ok_or(Error::InvalidStore)?
                .delete(&key)
                .await
        }
        .await)
    }

    async fn exists(&mut self, store: StoreHandle, key: String) -> Result<Result<bool, Error>> {
        Ok(async {
            self.stores
                .get(store)
                .ok_or(Error::InvalidStore)?
                .exists(&key)
                .await
        }
        .await)
    }

    async fn get_keys(&mut self, store: StoreHandle) -> Result<Result<Vec<String>, Error>> {
        Ok(async {
            self.stores
                .get(store)
                .ok_or(Error::InvalidStore)?
                .get_keys()
                .await
        }
        .await)
    }

    async fn close(&mut self, store: StoreHandle) -> Result<()> {
        self.stores.remove(store);
        Ok(())
    }
}

pub fn log_error(err: impl std::fmt::Debug) -> Error {
    tracing::warn!("key-value error: {err:?}");
    Error::Io(format!("{err:?}"))
}
