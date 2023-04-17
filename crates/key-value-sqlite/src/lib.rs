use anyhow::Result;
use once_cell::sync::OnceCell;
use rusqlite::Connection;
use spin_core::{async_trait, key_value::Error};
use spin_key_value::{log_error, Store, StoreManager};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::task;

pub enum DatabaseLocation {
    InMemory,
    Path(PathBuf),
}

pub struct KeyValueSqlite {
    location: DatabaseLocation,
    connection: OnceCell<Arc<Mutex<Connection>>>,
}

impl KeyValueSqlite {
    pub fn new(location: DatabaseLocation) -> Self {
        Self {
            location,
            connection: OnceCell::new(),
        }
    }
}

#[async_trait]
impl StoreManager for KeyValueSqlite {
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
}

struct SqliteStore {
    name: String,
    connection: Arc<Mutex<Connection>>,
}

#[async_trait]
impl Store for SqliteStore {
    async fn get(&self, key: &str) -> Result<Vec<u8>, Error> {
        task::block_in_place(|| {
            self.connection
                .lock()
                .unwrap()
                .prepare_cached("SELECT value FROM spin_key_value WHERE store=$1 AND key=$2")
                .map_err(log_error)?
                .query_map([&self.name, key], |row| row.get(0))
                .map_err(log_error)?
                .next()
                .ok_or(Error::NoSuchKey)?
                .map_err(log_error)
        })
    }

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

    async fn exists(&self, key: &str) -> Result<bool, Error> {
        match self.get(key).await {
            Ok(_) => Ok(true),
            Err(Error::NoSuchKey) => Ok(false),
            Err(e) => Err(e),
        }
    }

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
    use spin_core::key_value::Host;
    use spin_key_value::{DelegatingStoreManager, KeyValueDispatch};

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn all() -> Result<()> {
        let mut kv = KeyValueDispatch::new();
        kv.init(
            ["default", "foo"]
                .into_iter()
                .map(ToOwned::to_owned)
                .collect(),
            Arc::new(DelegatingStoreManager::new([(
                "default".to_owned(),
                Arc::new(KeyValueSqlite::new(DatabaseLocation::InMemory)) as _,
            )])),
        );

        assert!(matches!(
            kv.exists(42, "bar".to_owned()).await?,
            Err(Error::InvalidStore)
        ));

        assert!(matches!(
            kv.open("foo".to_owned()).await?,
            Err(Error::NoSuchStore)
        ));
        assert!(matches!(
            kv.open("forbidden".to_owned()).await?,
            Err(Error::AccessDenied)
        ));

        let store = kv.open("default".to_owned()).await??;

        assert!(!kv.exists(store, "bar".to_owned()).await??);

        assert!(matches!(
            kv.get(store, "bar".to_owned()).await?,
            Err(Error::NoSuchKey)
        ));

        kv.set(store, "bar".to_owned(), b"baz".to_vec()).await??;

        assert!(kv.exists(store, "bar".to_owned()).await??);

        assert_eq!(b"baz" as &[_], &kv.get(store, "bar".to_owned()).await??);

        kv.set(store, "bar".to_owned(), b"wow".to_vec()).await??;

        assert_eq!(b"wow" as &[_], &kv.get(store, "bar".to_owned()).await??);

        assert_eq!(&["bar".to_owned()] as &[_], &kv.get_keys(store).await??);

        kv.delete(store, "bar".to_owned()).await??;

        assert!(!kv.exists(store, "bar".to_owned()).await??);

        assert_eq!(&[] as &[String], &kv.get_keys(store).await??);

        assert!(matches!(
            kv.get(store, "bar".to_owned()).await?,
            Err(Error::NoSuchKey)
        ));

        kv.close(store).await?;

        assert!(matches!(
            kv.exists(store, "bar".to_owned()).await?,
            Err(Error::InvalidStore)
        ));

        Ok(())
    }
}
