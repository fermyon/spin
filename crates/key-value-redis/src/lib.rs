use anyhow::Result;
use redis::{aio::Connection, AsyncCommands};
use spin_core::{async_trait, key_value::Error};
use spin_key_value::{log_error, Store, StoreManager};
use std::sync::Arc;
use tokio::sync::{Mutex, OnceCell};

pub struct KeyValueRedis {
    database_url: String,
    connection: OnceCell<Arc<Mutex<Connection>>>,
}

impl KeyValueRedis {
    pub fn new(database_url: String) -> Self {
        Self {
            database_url,
            connection: OnceCell::new(),
        }
    }
}

#[async_trait]
impl StoreManager for KeyValueRedis {
    async fn get(&self, _name: &str) -> Result<Arc<dyn Store>, Error> {
        let connection = self
            .connection
            .get_or_try_init(|| async {
                redis::Client::open(self.database_url.clone())?
                    .get_async_connection()
                    .await
                    .map(Mutex::new)
                    .map(Arc::new)
            })
            .await
            .map_err(log_error)?;

        Ok(Arc::new(RedisStore {
            connection: connection.clone(),
        }))
    }
}

struct RedisStore {
    connection: Arc<Mutex<Connection>>,
}

#[async_trait]
impl Store for RedisStore {
    async fn get(&self, key: &str) -> Result<Vec<u8>, Error> {
        let mut conn = self.connection.lock().await;
        let result: Vec<u8> = conn.get(key).await.map_err(log_error)?;

        if result.is_empty() {
            Err(Error::NoSuchKey)
        } else {
            Ok(result)
        }
    }

    async fn set(&self, key: &str, value: &[u8]) -> Result<(), Error> {
        self.connection
            .lock()
            .await
            .set(key, value)
            .await
            .map_err(log_error)
    }

    async fn delete(&self, key: &str) -> Result<(), Error> {
        self.connection
            .lock()
            .await
            .del(key)
            .await
            .map_err(log_error)
    }

    async fn exists(&self, key: &str) -> Result<bool, Error> {
        self.connection
            .lock()
            .await
            .exists(key)
            .await
            .map_err(log_error)
    }

    async fn get_keys(&self) -> Result<Vec<String>, Error> {
        self.connection
            .lock()
            .await
            .keys("*")
            .await
            .map_err(log_error)
    }
}
