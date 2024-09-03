use anyhow::Result;
use once_cell::sync::OnceCell;
use rusqlite::Connection;
use spin_core::async_trait;
use spin_key_value::{log_error, Error, Store, StoreManager};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::task;
use tracing::{instrument, Level};

#[derive(Clone, Debug)]
pub enum DatabaseLocation {
    InMemory,
    Path(PathBuf),
}

pub struct KeyValueSqlite {
    location: DatabaseLocation,
    connection: OnceCell<Arc<Mutex<Connection>>>,
}

impl KeyValueSqlite {
    /// Create a new `KeyValueSqlite` store manager.
    ///
    /// If location is `DatabaseLocation::InMemory`, the database will be created in memory.
    /// If it's `DatabaseLocation::Path`, the database will be created at the specified path.
    /// Relative paths will be resolved against the current working directory.
    pub fn new(location: DatabaseLocation) -> Self {
        Self {
            location,
            connection: OnceCell::new(),
        }
    }
}

#[async_trait]
impl StoreManager for KeyValueSqlite {
    #[instrument(name = "spin_key_value_sqlite.get_store", skip(self), err(level = Level::INFO), fields(otel.kind = "client"))]
    async fn get(&self, name: &str) -> Result<Arc<dyn Store>, Error> {
        let connection = task::block_in_place(|| {
            self.connection.get_or_try_init(|| {
                let connection = match &self.location {
                    DatabaseLocation::InMemory => Connection::open_in_memory(),
                    DatabaseLocation::Path(path) => Connection::open(path),
                }
                .map_err(log_error)?;

                connection
                    .execute(
                        "CREATE TABLE IF NOT EXISTS spin_key_value (
                           store TEXT NOT NULL,
                           key   TEXT NOT NULL,
                           value BLOB NOT NULL,

                           PRIMARY KEY (store, key)
                        )",
                        [],
                    )
                    .map_err(log_error)?;

                Ok(Arc::new(Mutex::new(connection)))
            })
        })?;

        Ok(Arc::new(SqliteStore {
            name: name.to_owned(),
            connection: connection.clone(),
        }))
    }

    fn is_defined(&self, _store_name: &str) -> bool {
        true
    }

    fn summary(&self, _store_name: &str) -> Option<String> {
        Some(match &self.location {
            DatabaseLocation::InMemory => "a temporary in-memory store".into(),
            DatabaseLocation::Path(path) => format!("\"{}\"", path.display()),
        })
    }
}

struct SqliteStore {
    name: String,
    connection: Arc<Mutex<Connection>>,
}

#[async_trait]
impl Store for SqliteStore {
    #[instrument(name = "spin_key_value_sqlite.get", skip(self), err(level = Level::INFO), fields(otel.kind = "client"))]
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, Error> {
        task::block_in_place(|| {
            self.connection
                .lock()
                .unwrap()
                .prepare_cached("SELECT value FROM spin_key_value WHERE store=$1 AND key=$2")
                .map_err(log_error)?
                .query_map([&self.name, key], |row| row.get(0))
                .map_err(log_error)?
                .next()
                .transpose()
                .map_err(log_error)
        })
    }

    #[instrument(name = "spin_key_value_sqlite.set", skip(self, value), err(level = Level::INFO), fields(otel.kind = "client"))]
    async fn set(&self, key: &str, value: &[u8]) -> Result<(), Error> {
        task::block_in_place(|| {
            self.connection
                .lock()
                .unwrap()
                .prepare_cached(
                    "INSERT INTO spin_key_value (store, key, value) VALUES ($1, $2, $3)
                     ON CONFLICT(store, key) DO UPDATE SET value=$3",
                )
                .map_err(log_error)?
                .execute(rusqlite::params![&self.name, key, value])
                .map_err(log_error)
                .map(drop)
        })
    }

    #[instrument(name = "spin_key_value_sqlite.delete", skip(self), err(level = Level::INFO), fields(otel.kind = "client"))]
    async fn delete(&self, key: &str) -> Result<(), Error> {
        task::block_in_place(|| {
            self.connection
                .lock()
                .unwrap()
                .prepare_cached("DELETE FROM spin_key_value WHERE store=$1 AND key=$2")
                .map_err(log_error)?
                .execute([&self.name, key])
                .map_err(log_error)
                .map(drop)
        })
    }

    #[instrument(name = "spin_key_value_sqlite.exists", skip(self), err(level = Level::INFO), fields(otel.kind = "client"))]
    async fn exists(&self, key: &str) -> Result<bool, Error> {
        Ok(self.get(key).await?.is_some())
    }

    #[instrument(name = "spin_key_value_sqlite.get_keys", skip(self), err(level = Level::INFO), fields(otel.kind = "client"))]
    async fn get_keys(&self) -> Result<Vec<String>, Error> {
        task::block_in_place(|| {
            self.connection
                .lock()
                .unwrap()
                .prepare_cached("SELECT key FROM spin_key_value WHERE store=$1")
                .map_err(log_error)?
                .query_map([&self.name], |row| row.get(0))
                .map_err(log_error)?
                .map(|r| r.map_err(log_error))
                .collect()
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use spin_core::wasmtime::component::Resource;
    use spin_key_value::{DelegatingStoreManager, KeyValueDispatch};
    use spin_world::v2::key_value::HostStore;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn all() -> Result<()> {
        let mut kv = KeyValueDispatch::new();
        kv.init(
            ["default", "foo"]
                .into_iter()
                .map(ToOwned::to_owned)
                .collect(),
            Arc::new(DelegatingStoreManager::new(
                [(
                    "default".to_owned(),
                    Arc::new(KeyValueSqlite::new(DatabaseLocation::InMemory)) as _,
                )],
                Arc::new(|_: &str| -> Option<Arc<dyn StoreManager>> { None }),
            )),
        );

        assert!(matches!(
            kv.open("foo".to_owned()).await?,
            Err(Error::NoSuchStore)
        ));
        assert!(matches!(
            kv.open("forbidden".to_owned()).await?,
            Err(Error::AccessDenied)
        ));

        let store = kv.open("default".to_owned()).await??;
        let rep = store.rep();

        assert!(
            !kv.exists(Resource::new_own(rep), "bar".to_owned())
                .await??
        );

        assert!(matches!(
            kv.get(Resource::new_own(rep), "bar".to_owned()).await?,
            Ok(None)
        ));

        kv.set(Resource::new_own(rep), "bar".to_owned(), b"baz".to_vec())
            .await??;

        assert!(
            kv.exists(Resource::new_own(rep), "bar".to_owned())
                .await??
        );

        assert_eq!(
            Some(b"baz" as &[_]),
            kv.get(Resource::new_own(rep), "bar".to_owned())
                .await??
                .as_deref()
        );

        kv.set(Resource::new_own(rep), "bar".to_owned(), b"wow".to_vec())
            .await??;

        assert_eq!(
            Some(b"wow" as &[_]),
            kv.get(Resource::new_own(rep), "bar".to_owned())
                .await??
                .as_deref()
        );

        assert_eq!(
            &["bar".to_owned()] as &[_],
            &kv.get_keys(Resource::new_own(rep)).await??
        );

        kv.delete(Resource::new_own(rep), "bar".to_owned())
            .await??;

        assert!(
            !kv.exists(Resource::new_own(rep), "bar".to_owned())
                .await??
        );

        assert_eq!(
            &[] as &[String],
            &kv.get_keys(Resource::new_own(rep)).await??
        );

        assert!(matches!(
            kv.get(Resource::new_own(rep), "bar".to_owned()).await?,
            Ok(None)
        ));

        kv.drop(Resource::new_own(rep))?;

        Ok(())
    }
}
