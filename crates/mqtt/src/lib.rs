mod spin;

use crate::spin::SpinMqttExecutor;
use anyhow::Result;
use async_trait::async_trait;
use serde::{de::IgnoredAny, Deserialize, Serialize};
use spin_app::MetadataKey;
use spin_trigger::{cli::NoArgs, TriggerAppEngine, TriggerExecutor};
use std::{collections::HashMap, time::Duration};

const TRIGGER_METADATA_KEY: MetadataKey<TriggerMetadata> = MetadataKey::new("trigger");
pub(crate) type RuntimeData = ();
pub(crate) type Store = spin_core::Store<RuntimeData>;

// Spin Mqtt Trigger
pub struct MqttTrigger {
    engine: TriggerAppEngine<Self>,
    // Mqtt address to connect to
    address: String,
    // Mapping of subscription topics to component IDs
    topic_components: HashMap<String, String>,
}

/// Mqtt trigger configuration.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MqttTriggerConfig {
    // Component ID to invoke
    pub component: String,
    // Topic to subscribe to
    pub topic: String,
    // Trigger executor (currently unused)
    #[serde(default, skip_serializing)]
    pub executor: IgnoredAny,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TriggerMetadata {
    r#type: String,
    address: String,
}

#[async_trait]
impl TriggerExecutor for MqttTrigger {
    const TRIGGER_TYPE: &'static str = "mqtt";
    type RuntimeData = RuntimeData;
    type TriggerConfig = MqttTriggerConfig;
    type RunConfig = NoArgs;

    async fn new(engine: TriggerAppEngine<Self>) -> Result<Self> {
        let address = engine.app().require_metadata(TRIGGER_METADATA_KEY)?.address;

        let topic_components: HashMap<String, String> = engine
            .trigger_configs()
            .map(|(_, config)| (config.topic.clone(), config.component.clone()))
            .collect();

        Ok(Self {
            engine,
            address,
            topic_components,
        })
    }

    /// Run the Mqtt trigger indefinitely.
    async fn run(self, _config: Self::RunConfig) -> Result<()> {
        let address = &self.address;
        tracing::info!("Connecting to Mqtt server at {}", address);
        let client = paho_mqtt::Client::new(address.to_string())?;

        let conn_opts = paho_mqtt::ConnectOptionsBuilder::new()
            .keep_alive_interval(Duration::from_secs(60))
            .clean_session(true)
            .finalize();

        client.connect(conn_opts)?;

        for (topic, component) in self.topic_components.iter() {
            tracing::info!("Subscribing component {component:?} to topic {topic:?}");
            client.subscribe(topic, 1)?;
        }

        loop {
            // Wait for message to appear in the message channel.
            // Optimise this for parallel reads with single thread.

            match client.start_consuming().recv().unwrap() {
                Some(msg) => drop(self.handle(msg).await),
                None => {
                    println!("No message");
                    if !client.is_connected() {
                        println!("No Mqtt connection available");
                        break Ok(());
                    }
                }
            }
        }
    }
}

impl MqttTrigger {
    async fn handle(&self, msg: paho_mqtt::Message) -> Result<()> {
        let topic = msg.topic();
        tracing::info!("Received message on topic {:?}", topic);

        if let Some(component_id) = self.topic_components.get(topic) {
            tracing::trace!("Executing Mqtt component {component_id:?}");
            let executor = SpinMqttExecutor;
            executor
                .execute(&self.engine, component_id, topic, msg.payload())
                .await?
        }

        Ok(())
    }
}

/// The Mqtt executor trait.
/// All Mqtt executors must implement this trait.
#[async_trait]
pub trait MqttExecutor: Clone + Send + Sync + 'static {
    async fn execute(
        &self,
        engine: &TriggerAppEngine<MqttTrigger>,
        component_id: &str,
        topic: &str,
        payload: &[u8],
    ) -> Result<()>;
}

#[cfg(test)]
mod tests;
