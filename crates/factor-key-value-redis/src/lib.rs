use serde::Deserialize;
use spin_factor_key_value::MakeKeyValueStore;
use spin_key_value_redis::KeyValueRedis;
pub struct RedisKeyValueStore;

#[derive(Deserialize)]
pub struct RedisKeyValueRuntimeConfig {
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
