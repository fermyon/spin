use anyhow::{Context, Result};
use redis::{aio::ConnectionManager, parse_redis_url, AsyncCommands, Client, RedisError};
use spin_core::async_trait;
use spin_factor_key_value::{log_error, Cas, Error, Store, StoreManager, SwapError};
use std::sync::Arc;
use tokio::sync::OnceCell;
use url::Url;

pub struct KeyValueRedis {
    database_url: Url,
    connection: OnceCell<ConnectionManager>,
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
    async fn get(&self, _name: &str) -> Result<Arc<dyn Store>, Error> {
        let connection = self
            .connection
            .get_or_try_init(|| async {
                Client::open(self.database_url.clone())?
                    .get_connection_manager()
                    .await
            })
            .await
            .map_err(log_error)?;

        Ok(Arc::new(RedisStore {
            connection: connection.clone(),
            database_url: self.database_url.clone(),
        }))
    }

    fn is_defined(&self, _store_name: &str) -> bool {
        true
    }

    fn summary(&self, _store_name: &str) -> Option<String> {
        let redis::ConnectionInfo { addr, .. } = self.database_url.as_str().parse().ok()?;
        Some(format!("Redis at {addr}"))
    }
}

struct RedisStore {
    connection: ConnectionManager,
    database_url: Url,
}

struct CompareAndSwap {
    key: String,
    connection: ConnectionManager,
    bucket_rep: u32,
}

#[async_trait]
impl Store for RedisStore {
    async fn after_open(&self) -> Result<(), Error> {
        if let Err(_error) = self.connection.clone().ping::<()>().await {
            // If an IO error happens, ConnectionManager will start reconnection in the background
            // so we do not take any action and just pray re-connection will be successful.
        }
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, Error> {
        self.connection.clone().get(key).await.map_err(log_error)
    }

    async fn set(&self, key: &str, value: &[u8]) -> Result<(), Error> {
        self.connection
            .clone()
            .set(key, value)
            .await
            .map_err(log_error)
    }

    async fn delete(&self, key: &str) -> Result<(), Error> {
        self.connection.clone().del(key).await.map_err(log_error)
    }

    async fn exists(&self, key: &str) -> Result<bool, Error> {
        self.connection.clone().exists(key).await.map_err(log_error)
    }

    async fn get_keys(&self) -> Result<Vec<String>, Error> {
        self.connection.clone().keys("*").await.map_err(log_error)
    }

    async fn get_many(&self, keys: Vec<String>) -> Result<Vec<(String, Option<Vec<u8>>)>, Error> {
        self.connection.clone().keys(keys).await.map_err(log_error)
    }

    async fn set_many(&self, key_values: Vec<(String, Vec<u8>)>) -> Result<(), Error> {
        self.connection
            .clone()
            .mset(&key_values)
            .await
            .map_err(log_error)
    }

    async fn delete_many(&self, keys: Vec<String>) -> Result<(), Error> {
        self.connection.clone().del(keys).await.map_err(log_error)
    }

    async fn increment(&self, key: String, delta: i64) -> Result<i64, Error> {
        self.connection
            .clone()
            .incr(key, delta)
            .await
            .map_err(log_error)
    }

    /// `new_compare_and_swap` builds a new CAS structure giving it its own connection since Redis
    /// transactions are scoped to a connection and any WATCH should be dropped upon the drop of
    /// the connection.
    async fn new_compare_and_swap(
        &self,
        bucket_rep: u32,
        key: &str,
    ) -> Result<Arc<dyn Cas>, Error> {
        let cx = Client::open(self.database_url.clone())
            .map_err(log_error)?
            .get_connection_manager()
            .await
            .map_err(log_error)?;

        Ok(Arc::new(CompareAndSwap {
            key: key.to_string(),
            connection: cx,
            bucket_rep,
        }))
    }
}

#[async_trait]
impl Cas for CompareAndSwap {
    /// current will initiate a transaction by WATCH'ing a key in Redis, and then returning the
    /// current value for the key.
    async fn current(&self) -> Result<Option<Vec<u8>>, Error> {
        redis::cmd("WATCH")
            .arg(&self.key)
            .exec_async(&mut self.connection.clone())
            .await
            .map_err(log_error)?;
        self.connection
            .clone()
            .get(&self.key)
            .await
            .map_err(log_error)
    }

    /// swap will set the key to the new value only if the key has not changed. Afterward, the
    /// transaction will be terminated with an UNWATCH
    async fn swap(&self, value: Vec<u8>) -> Result<(), SwapError> {
        // Create transaction pipeline
        let mut transaction = redis::pipe();
        let res: Result<(), RedisError> = transaction
            .atomic()
            .set(&self.key, value)
            .query_async(&mut self.connection.clone())
            .await;

        redis::cmd("UNWATCH")
            .arg(&self.key)
            .exec_async(&mut self.connection.clone())
            .await
            .map_err(|err| SwapError::CasFailed(format!("{err:?}")))?;

        match res {
            Ok(_) => Ok(()),
            Err(err) => Err(SwapError::CasFailed(format!("{err:?}"))),
        }
    }

    async fn bucket_rep(&self) -> u32 {
        self.bucket_rep
    }

    async fn key(&self) -> String {
        self.key.clone()
    }
}
