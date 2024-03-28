use anyhow::Result;
use memcache::Client;
use spin_core::async_trait;
use spin_key_value::{log_error, Error, Store, StoreManager};
use std::sync::Arc;
use tokio::sync::OnceCell;

const NEVER_EXPIRE: u32 = 0;

pub struct KeyValueMemcached {
    urls: Vec<String>,
    pool_size: u32,
    client: OnceCell<Arc<Client>>,
}

impl KeyValueMemcached {
    pub fn new(addresses: Vec<String>, pool_size: Option<u32>) -> Result<Self> {
        Ok(Self {
            pool_size: pool_size.unwrap_or(32),
            urls: addresses,
            client: OnceCell::new(),
        })
    }
}

#[async_trait]
impl StoreManager for KeyValueMemcached {
    async fn get(&self, _name: &str) -> Result<Arc<dyn Store>, Error> {
        let client = self
            .client
            .get_or_try_init(|| async {
                Client::with_pool_size(self.urls.clone(), self.pool_size).map(Arc::new)
            })
            .await
            .map_err(log_error)?;

        Ok(Arc::new(MemcacheStore {
            client: client.clone(),
        }))
    }

    fn is_defined(&self, _store_name: &str) -> bool {
        true
    }
}

struct MemcacheStore {
    client: Arc<Client>,
}

#[async_trait]
impl Store for MemcacheStore {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, Error> {
        self.client.get(key).map_err(log_error)
    }

    async fn set(&self, key: &str, value: &[u8]) -> Result<(), Error> {
        self.client.set(key, value, NEVER_EXPIRE).map_err(log_error)
    }

    async fn delete(&self, key: &str) -> Result<(), Error> {
        self.client.delete(key).map(|_| ()).map_err(log_error)
    }

    async fn exists(&self, _key: &str) -> Result<bool, Error> {
        // memcache doesn't implement an "exists" api because it isn't actually
        // to check without getting the value. We require it, so implement via cas.
        // memcache uses a global incrementing value for `cas` so by setting the cas
        // value to zero, this should be safe in close to all cases without having
        // to worry about needlessly allocating memory for the response.
        //
        // TODO: test how this actually interacts with the rust lib and finish
        // let result = self.client.cas(key, 0, 0, 0);
        // match result {
        //     Ok(_) => Result::Ok(true),
        //     Err(err) => {
        //         match err {
        //             _ => Result::Err(log_error(err))
        //         }
        //     }
        // }
        Result::Err(Error::Other("not yet implemented".into()))
    }

    async fn get_keys(&self) -> Result<Vec<String>, Error> {
        // memcached is a distributed store with sharded keys. It can't reasonably
        // implement a `get_keys` function.
        Result::Err(Error::Other("get_keys unimplemented for memcached".into()))
    }
}
