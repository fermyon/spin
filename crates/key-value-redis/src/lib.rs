mod store;

use serde::Deserialize;
use spin_factor_key_value::runtime_config::spin::MakeKeyValueStore;
use store::KeyValueRedis;

/// A key-value store that uses Redis as the backend.
#[derive(Default)]
pub struct RedisKeyValueStore {
    _priv: (),
}

impl RedisKeyValueStore {
    /// Creates a new `RedisKeyValueStore`.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Runtime configuration for the Redis key-value store.
#[derive(Deserialize)]
pub struct RedisKeyValueRuntimeConfig {
    /// The URL of the Redis server.
    url: String,
}

impl MakeKeyValueStore for RedisKeyValueStore {
    const RUNTIME_CONFIG_TYPE: &'static str = "redis";

    type RuntimeConfig = RedisKeyValueRuntimeConfig;

    type StoreManager = KeyValueRedis;

    fn make_store(
        &self,
        runtime_config: Self::RuntimeConfig,
    ) -> anyhow::Result<Self::StoreManager> {
        KeyValueRedis::new(runtime_config.url)
    }
}
