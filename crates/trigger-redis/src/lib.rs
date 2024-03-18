//! Implementation for the Spin Redis engine.

mod spin;

use anyhow::{anyhow, Context, Result};
use futures::{future::join_all, StreamExt};
use redis::{Client, ConnectionLike};
use serde::{de::IgnoredAny, Deserialize, Serialize};
use spin_common::url::remove_credentials;
use spin_core::{async_trait, InstancePre};
use spin_trigger::{cli::NoArgs, TriggerAppEngine, TriggerExecutor};
use std::collections::HashMap;
use std::sync::Arc;

use crate::spin::SpinRedisExecutor;

pub(crate) type RuntimeData = ();
pub(crate) type Store = spin_core::Store<RuntimeData>;

type ChannelComponents = HashMap<String, Vec<String>>;
/// The Spin Redis trigger.
#[derive(Clone)]
pub struct RedisTrigger {
    engine: Arc<TriggerAppEngine<Self>>,
    // Mapping of server url with subscription channel and associated component IDs
    server_channels: HashMap<String, ChannelComponents>,
}

/// Redis trigger configuration.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RedisTriggerConfig {
    /// Component ID to invoke
    pub component: String,
    /// Channel to subscribe to
    pub channel: String,
    /// optional overide address for trigger
    pub address: Option<String>,
    /// Trigger executor (currently unused)
    #[serde(default, skip_serializing)]
    pub executor: IgnoredAny,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct TriggerMetadata {
    address: String,
}

#[async_trait]
impl TriggerExecutor for RedisTrigger {
    const TRIGGER_TYPE: &'static str = "redis";
    type RuntimeData = RuntimeData;
    type TriggerConfig = RedisTriggerConfig;
    type RunConfig = NoArgs;
    type InstancePre = InstancePre<RuntimeData>;

    async fn new(engine: TriggerAppEngine<Self>) -> Result<Self> {
        let default_address: String = engine
            .trigger_metadata::<TriggerMetadata>()?
            .unwrap_or_default()
            .address;
        let default_address_expr = spin_expressions::Template::new(default_address)?;
        let default_address = engine.resolve_template(&default_address_expr)?;

        let mut server_channels: HashMap<String, ChannelComponents> = HashMap::new();

        for (_, config) in engine.trigger_configs() {
            let address = config.address.clone().unwrap_or(default_address.clone());
            let address_expr = spin_expressions::Template::new(address)?;
            let address = engine.resolve_template(&address_expr)?;
            let server = server_channels.entry(address).or_default();
            let channel_expr = spin_expressions::Template::new(config.channel.as_str())?;
            let channel = engine.resolve_template(&channel_expr)?;
            server
                .entry(channel)
                .or_default()
                .push(config.component.clone());
        }
        Ok(Self {
            engine: Arc::new(engine),
            server_channels,
        })
    }

    /// Run the Redis trigger indefinitely.
    async fn run(self, _config: Self::RunConfig) -> Result<()> {
        let tasks: Vec<_> = self
            .server_channels
            .clone()
            .into_iter()
            .map(|(server_address, channel_components)| {
                let trigger = self.clone();
                tokio::spawn(async move {
                    trigger
                        .run_listener(server_address.clone(), channel_components.clone())
                        .await
                })
            })
            .collect();

        // wait for the first handle to be returned and drop the rest
        let (result, _, rest) = futures::future::select_all(tasks).await;

        drop(rest);

        result?
    }
}

impl RedisTrigger {
    // Handle the message.
    async fn handle(
        &self,
        address: &str,
        channel_components: &ChannelComponents,
        msg: redis::Msg,
    ) -> Result<()> {
        let channel = msg.get_channel_name();
        tracing::info!("Received message on channel {address}:{:?}", channel);

        if let Some(component_ids) = channel_components.get(channel) {
            let futures = component_ids.iter().map(|id| {
                tracing::trace!("Executing Redis component {id:?}");
                SpinRedisExecutor.execute(&self.engine, id, channel, msg.get_payload_bytes())
            });
            let results: Vec<_> = join_all(futures).await.into_iter().collect();
            let errors = results
                .into_iter()
                .filter_map(|r| r.err())
                .collect::<Vec<_>>();
            if !errors.is_empty() {
                return Err(anyhow!("{errors:#?}"));
            }
        } else {
            tracing::debug!("No subscription found for {:?}", channel);
        }
        Ok(())
    }

    async fn run_listener(
        &self,
        address: String,
        channel_components: ChannelComponents,
    ) -> Result<()> {
        tracing::info!("Connecting to Redis server at {}", address);
        let mut client = Client::open(address.to_string())?;
        let mut pubsub = client
            .get_async_connection()
            .await
            .with_context(|| anyhow!("Redis trigger failed to connect to {}", address))?
            .into_pubsub();

        let sanitised_addr = remove_credentials(&address)?;
        println!("Active Channels on {sanitised_addr}:");
        // Subscribe to channels
        for (channel, component) in channel_components.iter() {
            tracing::info!("Subscribing component {component:?} to channel {channel:?}");
            pubsub.subscribe(channel).await?;
            println!("\t{sanitised_addr}:{channel}: [{}]", component.join(","));
        }

        let mut stream = pubsub.on_message();
        loop {
            match stream.next().await {
                Some(msg) => {
                    if let Err(err) = self.handle(&address, &channel_components, msg).await {
                        tracing::warn!("Error handling message: {err}");
                    }
                }
                None => {
                    tracing::trace!("Empty message");
                    if !client.check_connection() {
                        tracing::info!("No Redis connection available");
                        println!("Disconnected from {address}");
                        break;
                    }
                }
            };
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
