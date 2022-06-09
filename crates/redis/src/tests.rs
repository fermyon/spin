use super::*;
use anyhow::Result;
use redis::{Msg, Value};
use spin_manifest::{RedisConfig, RedisExecutor};
use spin_testing::TestConfig;
use spin_trigger::TriggerExecutorBuilder;
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

fn create_trigger_event(channel: &str, payload: &str) -> redis::Msg {
    Msg::from_value(&redis::Value::Bulk(vec![
        Value::Data("message".into()),
        Value::Data(channel.into()),
        Value::Data(payload.into()),
    ]))
    .unwrap()
}

#[ignore]
#[tokio::test]
async fn test_pubsub() -> Result<()> {
    init();

    let mut cfg = TestConfig::default();
    cfg.test_program("redis-rust.wasm")
        .redis_trigger(RedisConfig {
            channel: "messages".to_string(),
            executor: Some(RedisExecutor::Spin),
        });
    let app = cfg.build_application();

    let trigger: RedisTrigger = TriggerExecutorBuilder::new(app).build().await?;

    let msg = create_trigger_event("messages", "hello");
    trigger.handle(msg).await?;

    Ok(())
}
