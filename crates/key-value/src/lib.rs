use anyhow::Result;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use table::Table;
use wit_bindgen_wasmtime::async_trait;

mod host_component;
mod table;

pub use host_component::KeyValueComponent;

wit_bindgen_wasmtime::export!({paths: ["../../wit/ephemeral/key-value.wit"], async: *});

pub use key_value::{Error, KeyValue, Store};

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error {}

#[async_trait]
pub trait Impl: Sync + Send {
    async fn open(&self, name: &str) -> Result<Box<dyn ImplStore>, Error>;
}

#[async_trait]
pub trait ImplStore: Sync + Send {
    async fn get(&self, key: &str) -> Result<Vec<u8>, Error>;

    async fn set(&self, key: &str, value: &[u8]) -> Result<(), Error>;

    async fn delete(&self, key: &str) -> Result<(), Error>;

    async fn exists(&self, key: &str) -> Result<bool, Error>;

    async fn get_keys(&self) -> Result<Vec<String>, Error>;
}

pub struct KeyValueDispatch {
    pub allowed_stores: HashSet<String>,
    impls: Arc<HashMap<String, Box<dyn Impl>>>,
    stores: Table<Box<dyn ImplStore>>,
}

impl KeyValueDispatch {
    pub fn new(impls: Arc<HashMap<String, Box<dyn Impl>>>) -> Self {
        Self {
            allowed_stores: HashSet::new(),
            impls,
            stores: Table::new(),
        }
    }
}

#[async_trait]
impl KeyValue for KeyValueDispatch {
    async fn open(&mut self, name: &str) -> Result<Store, Error> {
        if self.allowed_stores.contains(name) {
            self.stores
                .push(
                    self.impls
                        .get(name)
                        .ok_or(Error::NoSuchStore)?
                        .open(name)
                        .await?,
                )
                .map_err(|()| Error::StoreTableFull)
        } else {
            Err(Error::AccessDenied)
        }
    }

    async fn get(&mut self, store: Store, key: &str) -> Result<Vec<u8>, Error> {
        self.stores
            .get(store)
            .ok_or(Error::InvalidStore)?
            .get(key)
            .await
    }

    async fn set(&mut self, store: Store, key: &str, value: &[u8]) -> Result<(), Error> {
        self.stores
            .get(store)
            .ok_or(Error::InvalidStore)?
            .set(key, value)
            .await
    }

    async fn delete(&mut self, store: Store, key: &str) -> Result<(), Error> {
        self.stores
            .get(store)
            .ok_or(Error::InvalidStore)?
            .delete(key)
            .await
    }

    async fn exists(&mut self, store: Store, key: &str) -> Result<bool, Error> {
        self.stores
            .get(store)
            .ok_or(Error::InvalidStore)?
            .exists(key)
            .await
    }

    async fn get_keys(&mut self, store: Store) -> Result<Vec<String>, Error> {
        self.stores
            .get(store)
            .ok_or(Error::InvalidStore)?
            .get_keys()
            .await
    }

    async fn close(&mut self, store: Store) {
        self.stores.remove(store);
    }
}

pub fn log_error(err: impl std::fmt::Debug) -> Error {
    tracing::warn!("key-value error: {err:?}");
    Error::Io(format!("{err:?}"))
}
