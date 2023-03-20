//! Implementation for the Spin Redis engine.

mod spin;

use std::collections::HashMap;

use anyhow::{anyhow, Context, Result};
use futures::StreamExt;
use redis::{Client, ConnectionLike};
use serde::{de::IgnoredAny, Deserialize, Serialize};
use spin_app::MetadataKey;
use spin_core::async_trait;
use spin_trigger::{cli::NoArgs, TriggerAppEngine, TriggerExecutor};

use crate::spin::SpinRedisExecutor;

const TRIGGER_METADATA_KEY: MetadataKey<TriggerMetadata> = MetadataKey::new("trigger");

pub(crate) type RuntimeData = ();
pub(crate) type Store = spin_core::Store<RuntimeData>;

/// The Spin Redis trigger.
pub struct RedisTrigger {
    engine: TriggerAppEngine<Self>,
    // Redis address to connect to
    address: String,
    // Mapping of subscription channels to component IDs
    channel_components: HashMap<String, String>,
}

/// Redis trigger configuration.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RedisTriggerConfig {
    /// Component ID to invoke
    pub component: String,
    /// Channel to subscribe to
    pub channel: String,
    /// Trigger executor (currently unused)
    #[serde(default, skip_serializing)]
    pub executor: IgnoredAny,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct TriggerMetadata {
    r#type: String,
    address: String,
}

#[async_trait]
impl TriggerExecutor for RedisTrigger {
    const TRIGGER_TYPE: &'static str = "redis";
    type RuntimeData = RuntimeData;
    type TriggerConfig = RedisTriggerConfig;
    type RunConfig = NoArgs;

    async fn new(engine: TriggerAppEngine<Self>) -> Result<Self> {
        let address = engine.app().require_metadata(TRIGGER_METADATA_KEY)?.address;

        let channel_components = engine
            .trigger_configs()
            .map(|(_, config)| (config.channel.clone(), config.component.clone()))
            .collect();

        Ok(Self {
            engine,
            address,
            channel_components,
        })
    }

    /// Run the Redis trigger indefinitely.
    async fn run(self, _config: Self::RunConfig) -> Result<()> {
        let address = &self.address;

        tracing::info!("Connecting to Redis server at {}", address);
        let mut client = Client::open(address.to_string())?;
        let mut pubsub = client
            .get_async_connection()
            .await
            .with_context(|| anyhow!("Redis trigger failed to connect to {}", address))?
            .into_pubsub();

        // Subscribe to channels
        for (channel, component) in self.channel_components.iter() {
            tracing::info!("Subscribing component {component:?} to channel {channel:?}");
            pubsub.subscribe(channel).await?;
        }

        let mut stream = pubsub.on_message();
        loop {
            match stream.next().await {
                Some(msg) => drop(self.handle(msg).await),
                None => {
                    tracing::trace!("Empty message");
                    if !client.check_connection() {
                        tracing::info!("No Redis connection available");
                        break Ok(());
                    }
                }
            };
        }
    }
}

impl RedisTrigger {
    // Handle the message.
    async fn handle(&self, msg: redis::Msg) -> Result<()> {
        let channel = msg.get_channel_name();
        tracing::info!("Received message on channel {:?}", channel);

        if let Some(component_id) = self.channel_components.get(channel) {
            tracing::trace!("Executing Redis component {component_id:?}");
            let executor = SpinRedisExecutor;
            executor
                .execute(&self.engine, component_id, channel, msg.get_payload_bytes())
                .await?
        } else {
            tracing::debug!("No subscription found for {:?}", channel);
        }

        Ok(())
    }
}

/// The Redis executor trait.
/// All Redis executors must implement this trait.
#[async_trait]
pub(crate) trait RedisExecutor: Clone + Send + Sync + 'static {
    async fn execute(
        &self,
        engine: &TriggerAppEngine<RedisTrigger>,
        component_id: &str,
        channel: &str,
        payload: &[u8],
    ) -> Result<()>;
}

#[cfg(test)]
mod tests;
