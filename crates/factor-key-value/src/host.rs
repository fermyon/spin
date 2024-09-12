use crate::util::EmptyStoreManager;
use anyhow::{Context, Result};
use spin_core::{async_trait, wasmtime::component::Resource};
use spin_world::v2::key_value;
use std::{collections::HashSet, sync::Arc};
use table::Table;
use tracing::{instrument, Level};

const DEFAULT_STORE_TABLE_CAPACITY: u32 = 256;

pub use key_value::Error;

#[async_trait]
pub trait StoreManager: Sync + Send {
    async fn get(&self, name: &str) -> Result<Arc<dyn Store>, Error>;
    fn is_defined(&self, store_name: &str) -> bool;

    /// A human-readable summary of the given store's configuration
    ///
    /// Example: "Redis at localhost:1234"
    fn summary(&self, store_name: &str) -> Option<String> {
        let _ = store_name;
        None
    }
}

#[async_trait]
pub trait Store: Sync + Send {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, Error>;
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

    pub fn get_store(&self, store: Resource<key_value::Store>) -> anyhow::Result<&Arc<dyn Store>> {
        self.stores.get(store.rep()).context("invalid store")
    }

    pub fn allowed_stores(&self) -> &HashSet<String> {
        &self.allowed_stores
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
    #[instrument(name = "spin_key_value.open", skip(self), err(level = Level::INFO), fields(otel.kind = "client", kv.backend=self.manager.summary(&name).unwrap_or("unknown".to_string())))]
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

    #[instrument(name = "spin_key_value.get", skip(self, store), err(level = Level::INFO), fields(otel.kind = "client"))]
    async fn get(
        &mut self,
        store: Resource<key_value::Store>,
        key: String,
    ) -> Result<Result<Option<Vec<u8>>, Error>> {
        let store = self.get_store(store)?;
        Ok(store.get(&key).await)
    }

    #[instrument(name = "spin_key_value.set", skip(self, store, value), err(level = Level::INFO), fields(otel.kind = "client"))]
    async fn set(
        &mut self,
        store: Resource<key_value::Store>,
        key: String,
        value: Vec<u8>,
    ) -> Result<Result<(), Error>> {
        let store = self.get_store(store)?;
        Ok(store.set(&key, &value).await)
    }

    #[instrument(name = "spin_key_value.delete", skip(self, store), err(level = Level::INFO), fields(otel.kind = "client"))]
    async fn delete(
        &mut self,
        store: Resource<key_value::Store>,
        key: String,
    ) -> Result<Result<(), Error>> {
        let store = self.get_store(store)?;
        Ok(store.delete(&key).await)
    }

    #[instrument(name = "spin_key_value.exists", skip(self, store), err(level = Level::INFO), fields(otel.kind = "client"))]
    async fn exists(
        &mut self,
        store: Resource<key_value::Store>,
        key: String,
    ) -> Result<Result<bool, Error>> {
        let store = self.get_store(store)?;
        Ok(store.exists(&key).await)
    }

    #[instrument(name = "spin_key_value.get_keys", skip(self, store), err(level = Level::INFO), fields(otel.kind = "client"))]
    async fn get_keys(
        &mut self,
        store: Resource<key_value::Store>,
    ) -> Result<Result<Vec<String>, Error>> {
        let store = self.get_store(store)?;
        Ok(store.get_keys().await)
    }

    fn drop(&mut self, store: Resource<key_value::Store>) -> Result<()> {
        self.stores.remove(store.rep());
        Ok(())
    }
}

pub fn log_error(err: impl std::fmt::Debug) -> Error {
    tracing::warn!("key-value error: {err:?}");
    Error::Other(format!("{err:?}"))
}

use spin_world::v1::key_value::Error as LegacyError;

fn to_legacy_error(value: key_value::Error) -> LegacyError {
    match value {
        Error::StoreTableFull => LegacyError::StoreTableFull,
        Error::NoSuchStore => LegacyError::NoSuchStore,
        Error::AccessDenied => LegacyError::AccessDenied,
        Error::Other(s) => LegacyError::Io(s),
    }
}

#[async_trait]
impl spin_world::v1::key_value::Host for KeyValueDispatch {
    async fn open(&mut self, name: String) -> Result<Result<u32, LegacyError>> {
        let result = <Self as key_value::HostStore>::open(self, name).await?;
        Ok(result.map_err(to_legacy_error).map(|s| s.rep()))
    }

    async fn get(&mut self, store: u32, key: String) -> Result<Result<Vec<u8>, LegacyError>> {
        let this = Resource::new_borrow(store);
        let result = <Self as key_value::HostStore>::get(self, this, key).await?;
        Ok(result
            .map_err(to_legacy_error)
            .and_then(|v| v.ok_or(LegacyError::NoSuchKey)))
    }

    async fn set(
        &mut self,
        store: u32,
        key: String,
        value: Vec<u8>,
    ) -> Result<Result<(), LegacyError>> {
        let this = Resource::new_borrow(store);
        let result = <Self as key_value::HostStore>::set(self, this, key, value).await?;
        Ok(result.map_err(to_legacy_error))
    }

    async fn delete(&mut self, store: u32, key: String) -> Result<Result<(), LegacyError>> {
        let this = Resource::new_borrow(store);
        let result = <Self as key_value::HostStore>::delete(self, this, key).await?;
        Ok(result.map_err(to_legacy_error))
    }

    async fn exists(&mut self, store: u32, key: String) -> Result<Result<bool, LegacyError>> {
        let this = Resource::new_borrow(store);
        let result = <Self as key_value::HostStore>::exists(self, this, key).await?;
        Ok(result.map_err(to_legacy_error))
    }

    async fn get_keys(&mut self, store: u32) -> Result<Result<Vec<String>, LegacyError>> {
        let this = Resource::new_borrow(store);
        let result = <Self as key_value::HostStore>::get_keys(self, this).await?;
        Ok(result.map_err(to_legacy_error))
    }

    async fn close(&mut self, store: u32) -> Result<()> {
        let this = Resource::new_borrow(store);
        <Self as key_value::HostStore>::drop(self, this)
    }
}
