use super::*;
use anyhow::Result;
use redis::{Msg, Value};
use spin_testing::{tokio, RedisTestConfig};

fn create_trigger_event(channel: &str, payload: &str) -> redis::Msg {
    Msg::from_value(&redis::Value::Bulk(vec![
        Value::Data("message".into()),
        Value::Data(channel.into()),
        Value::Data(payload.into()),
    ]))
    .unwrap()
}

#[tokio::test]
async fn test_pubsub() -> Result<()> {
    let trigger: RedisTrigger = RedisTestConfig::default()
        .test_program("redis-rust.wasm")
        .build_trigger("messages")
        .await;

    let msg = create_trigger_event("messages", "hello");
    trigger.handle(msg).await?;

    Ok(())
}
