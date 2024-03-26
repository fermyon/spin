use anyhow::{Context, Result};
use redis::{aio::Connection, parse_redis_url, AsyncCommands};
use spin_core::async_trait;
use spin_key_value::{log_error, Error, Store, StoreManager};
use std::sync::Arc;
use tokio::sync::{Mutex, OnceCell};
use tracing::{instrument, Level};
use url::Url;

pub struct KeyValueRedis {
    database_url: Url,
    connection: OnceCell<Arc<Mutex<Connection>>>,
}

impl KeyValueRedis {
    pub fn new(address: String) -> Result<Self> {
        let database_url = parse_redis_url(&address).context("Invalid Redis URL")?;

        Ok(Self {
            database_url,
            connection: OnceCell::new(),
        })
    }
}

#[async_trait]
impl StoreManager for KeyValueRedis {
    #[instrument(name = "get_redis_kv_store", skip(self), err(level = Level::INFO), fields(otel.kind = "client"))]
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

    #[instrument(name = "is_defined_redis_kv_store", skip(self), level = Level::DEBUG)]
    fn is_defined(&self, _store_name: &str) -> bool {
        true
    }
}

struct RedisStore {
    connection: Arc<Mutex<Connection>>,
}

#[async_trait]
impl Store for RedisStore {
    #[instrument(name = "get_value_redis_kv", skip(self), err(level = Level::INFO), fields(otel.kind = "client"))]
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, Error> {
        let mut conn = self.connection.lock().await;
        conn.get(key).await.map_err(log_error)
    }

    #[instrument(name = "set_value_redis_kv", skip(self, value), err(level = Level::INFO), fields(otel.kind = "client"))]
    async fn set(&self, key: &str, value: &[u8]) -> Result<(), Error> {
        self.connection
            .lock()
            .await
            .set(key, value)
            .await
            .map_err(log_error)
    }

    #[instrument(name = "delete_value_redis_kv", skip(self), err(level = Level::INFO), fields(otel.kind = "client"))]
    async fn delete(&self, key: &str) -> Result<(), Error> {
        self.connection
            .lock()
            .await
            .del(key)
            .await
            .map_err(log_error)
    }

    #[instrument(name = "key_exists_redis_kv", skip(self), err(level = Level::INFO), fields(otel.kind = "client"))]
    async fn exists(&self, key: &str) -> Result<bool, Error> {
        self.connection
            .lock()
            .await
            .exists(key)
            .await
            .map_err(log_error)
    }

    #[instrument(name = "get_keys_redis_kv", skip(self), err(level = Level::INFO), fields(otel.kind = "client"))]
    async fn get_keys(&self) -> Result<Vec<String>, Error> {
        self.connection
            .lock()
            .await
            .keys("*")
            .await
            .map_err(log_error)
    }
}
