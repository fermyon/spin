use super::*;
use anyhow::Result;
use paho_mqtt::Message;
use spin_testing::{tokio, MqttTestConfig};

fn create_trigger_event(topic: &str, payload: &str) -> paho_mqtt::Message {
    Message::new(topic, payload, 0)
}

#[tokio::test]
async fn test_pubsub() -> Result<()> {
    let trigger: MqttTrigger = MqttTestConfig::default()
        .test_program("mqtt-rust.wasm")
        .build_trigger("messages")
        .await;
    let msg = create_trigger_event("messages", "hello");
    trigger.handle(msg).await?;

    Ok(())
}
