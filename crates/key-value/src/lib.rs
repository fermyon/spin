use anyhow::Result;
use spin_app::MetadataKey;
use spin_core::{async_trait, wasmtime::component::Resource};
use spin_world::v2::key_value::{self, HostStore};
use std::{collections::HashSet, sync::Arc};
use table::Table;

mod host_component;
pub mod table;
mod util;

pub use host_component::{manager, KeyValueComponent, LegacyKeyValueComponent};
pub use util::{CachingStoreManager, DelegatingStoreManager, EmptyStoreManager};

pub const KEY_VALUE_STORES_KEY: MetadataKey<Vec<String>> = MetadataKey::new("key_value_stores");

const DEFAULT_STORE_TABLE_CAPACITY: u32 = 256;

pub use key_value::{Error, Store as StoreHandle};

#[async_trait]
pub trait StoreManager: Sync + Send {
    async fn get(&self, name: &str) -> Result<Arc<dyn Store>, Error>;
    fn is_defined(&self, store_name: &str) -> bool;
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
impl key_value::Host for KeyValueDispatch {}

#[async_trait]
impl key_value::HostStore for KeyValueDispatch {
    async fn open(&mut self, name: String) -> Result<Result<Resource<key_value::Store>, Error>> {
        Ok(async {
            if self.allowed_stores.contains(&name) {
                let store = self
                    .stores
                    .push(self.manager.get(&name).await?)
                    .map_err(|()| Error::StoreTableFull)?;
                Ok(Resource::new_own(store))
            } else {
                Err(Error::AccessDenied)
            }
        }
        .await)
    }

    async fn get(
        &mut self,
        store: Resource<key_value::Store>,
        key: String,
    ) -> Result<Result<Vec<u8>, Error>> {
        Ok(async {
            self.stores
                .get(store.rep())
                .ok_or(Error::InvalidStore)?
                .get(&key)
                .await
        }
        .await)
    }

    async fn set(
        &mut self,
        store: Resource<key_value::Store>,
        key: String,
        value: Vec<u8>,
    ) -> Result<Result<(), Error>> {
        Ok(async {
            self.stores
                .get(store.rep())
                .ok_or(Error::InvalidStore)?
                .set(&key, &value)
                .await
        }
        .await)
    }

    async fn delete(
        &mut self,
        store: Resource<key_value::Store>,
        key: String,
    ) -> Result<Result<(), Error>> {
        Ok(async {
            self.stores
                .get(store.rep())
                .ok_or(Error::InvalidStore)?
                .delete(&key)
                .await
        }
        .await)
    }

    async fn exists(
        &mut self,
        store: Resource<key_value::Store>,
        key: String,
    ) -> Result<Result<bool, Error>> {
        Ok(async {
            self.stores
                .get(store.rep())
                .ok_or(Error::InvalidStore)?
                .exists(&key)
                .await
        }
        .await)
    }

    async fn get_keys(
        &mut self,
        store: Resource<key_value::Store>,
    ) -> Result<Result<Vec<String>, Error>> {
        Ok(async {
            self.stores
                .get(store.rep())
                .ok_or(Error::InvalidStore)?
                .get_keys()
                .await
        }
        .await)
    }

    fn drop(&mut self, store: Resource<key_value::Store>) -> Result<()> {
        self.stores.remove(store.rep());
        Ok(())
    }
}

pub fn log_error(err: impl std::fmt::Debug) -> Error {
    tracing::warn!("key-value error: {err:?}");
    Error::Io(format!("{err:?}"))
}

use spin_world::v1::key_value::Error as LegacyError;

pub struct LegacyDispatch(KeyValueDispatch);

fn to_legacy_error(value: key_value::Error) -> LegacyError {
    match value {
        Error::StoreTableFull => LegacyError::StoreTableFull,
        Error::NoSuchStore => LegacyError::NoSuchStore,
        Error::AccessDenied => LegacyError::AccessDenied,
        Error::InvalidStore => LegacyError::InvalidStore,
        Error::NoSuchKey => LegacyError::NoSuchKey,
        Error::Io(s) => LegacyError::Io(s),
    }
}

#[async_trait]
impl spin_world::v1::key_value::Host for LegacyDispatch {
    async fn open(&mut self, name: String) -> Result<Result<u32, LegacyError>> {
        let result = self.0.open(name).await?;
        Ok(result.map_err(to_legacy_error).map(|s| s.rep()))
    }

    async fn get(&mut self, store: u32, key: String) -> Result<Result<Vec<u8>, LegacyError>> {
        let this = Resource::new_borrow(store);
        let result = self.0.get(this, key).await?;
        Ok(result.map_err(to_legacy_error))
    }

    async fn set(
        &mut self,
        store: u32,
        key: String,
        value: Vec<u8>,
    ) -> Result<Result<(), LegacyError>> {
        let this = Resource::new_borrow(store);
        let result = self.0.set(this, key, value).await?;
        Ok(result.map_err(to_legacy_error))
    }

    async fn delete(&mut self, store: u32, key: String) -> Result<Result<(), LegacyError>> {
        let this = Resource::new_borrow(store);
        let result = self.0.delete(this, key).await?;
        Ok(result.map_err(to_legacy_error))
    }

    async fn exists(&mut self, store: u32, key: String) -> Result<Result<bool, LegacyError>> {
        let this = Resource::new_borrow(store);
        let result = self.0.exists(this, key).await?;
        Ok(result.map_err(to_legacy_error))
    }

    async fn get_keys(&mut self, store: u32) -> Result<Result<Vec<String>, LegacyError>> {
        let this = Resource::new_borrow(store);
        let result = self.0.get_keys(this).await?;
        Ok(result.map_err(to_legacy_error))
    }

    async fn close(&mut self, store: u32) -> Result<()> {
        let this = Resource::new_borrow(store);
        self.0.drop(this)
    }
}
