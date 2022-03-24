use super::*;
use anyhow::Result;
use spin_config::{RedisConfig, RedisExecutor};
use spin_testing::TestConfig;
use std::sync::Once;

static LOGGER: Once = Once::new();

/// We can only initialize the tracing subscriber once per crate.
pub(crate) fn init() {
    LOGGER.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .init();
    });
}

#[tokio::test]
#[allow(unused)]
async fn test_pubsub() -> Result<()> {
    init();

    let cfg = TestConfig::default()
        .test_program("redis-rust.wasm")
        .redis_trigger(RedisConfig {
            channel: "messages".to_string(),
            executor: Some(RedisExecutor::Spin),
        })
        .build_configuration();

    let trigger = RedisTrigger::new(cfg, None, None).await?;

    // TODO
    // use redis::{FromRedisValue, Msg, Value};
    // let val = FromRedisValue::from_redis_value(&Value::Data("hello".into()))?;
    // let msg = Msg::from_value(&val).unwrap();
    // trigger.handle(msg).await?;

    Ok(())
}
