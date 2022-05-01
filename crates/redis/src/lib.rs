//! Implementation for the Spin Redis engine.

mod spin;

use crate::spin::SpinRedisExecutor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::StreamExt;
use redis::{Client, ConnectionLike};
use spin_engine::Builder;
use spin_manifest::{
    Application, ComponentMap, CoreComponent, RedisConfig, RedisTriggerConfiguration,
};
use spin_redis::SpinRedisData;
use spin_trigger::Trigger;
use std::{collections::HashMap, sync::Arc};

wit_bindgen_wasmtime::import!("../../wit/ephemeral/spin-redis.wit");

type ExecutionContext = spin_engine::ExecutionContext<SpinRedisData>;
type RuntimeContext = spin_engine::RuntimeContext<SpinRedisData>;

/// The Spin Redis trigger.
#[derive(Clone)]
pub struct RedisTrigger {
    /// Trigger configuration.
    trigger_config: RedisTriggerConfiguration,
    /// Component trigger configurations.
    component_triggers: ComponentMap<RedisConfig>,
    /// Spin execution context.
    engine: Arc<ExecutionContext>,
    /// Map from channel name to tuple of component name & index.
    subscriptions: HashMap<String, usize>,
}

#[async_trait]
impl Trigger for RedisTrigger {
    type ContextData = SpinRedisData;
    type Config = RedisTriggerConfiguration;
    type ComponentConfig = RedisConfig;
    type RuntimeConfig = ();
    type TriggerExtra = HashMap<String, usize>;

    fn new(
        execution_context: ExecutionContext,
        config: Self::Config,
        component_configs: ComponentMap<Self::ComponentConfig>,
        trigger_extra: Self::TriggerExtra,
    ) -> Result<Self> {
        Ok(Self {
            trigger_config: config,
            component_triggers: component_configs,
            engine: Arc::new(execution_context),
            subscriptions: trigger_extra,
        })
    }

    fn build_trigger_extra(app: Application<CoreComponent>) -> Result<Self::TriggerExtra> {
        Ok(app
            .components
            .iter()
            .enumerate()
            .filter_map(|(idx, c)| {
                app.component_triggers
                    .get(c)
                    .map(|c| (c.as_redis().unwrap().channel.clone(), idx))
            })
            .collect())
    }

    /// Run the Redis trigger indefinitely.
    async fn run(&self, _: Self::RuntimeConfig) -> Result<()> {
        let address = self.trigger_config.address.as_str();

        log::info!("Connecting to Redis server at {}", address);
        let mut client = Client::open(address.to_string())?;
        let mut pubsub = client.get_async_connection().await?.into_pubsub();

        // Subscribe to channels
        for (subscription, idx) in self.subscriptions.iter() {
            let name = &self.engine.config.components[*idx].id;
            log::info!(
                "Subscribed component #{} ({}) to channel: {}",
                idx,
                name,
                subscription
            );
            pubsub.subscribe(subscription).await?;
        }

        let mut stream = pubsub.on_message();
        loop {
            match stream.next().await {
                Some(msg) => drop(self.handle(msg).await),
                None => {
                    log::trace!("Empty message");
                    if !client.check_connection() {
                        log::info!("No Redis connection available");
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
        log::info!("Received message on channel {:?}", channel);

        if let Some(idx) = self.subscriptions.get(channel).copied() {
            let component = &self.engine.config.components[idx];
            let executor = self
                .component_triggers
                .get(component)
                .and_then(|t| t.executor.clone())
                .unwrap_or_default();

            match executor {
                spin_manifest::RedisExecutor::Spin => {
                    log::trace!("Executing Spin Redis component {}", component.id);
                    let executor = SpinRedisExecutor;
                    executor
                        .execute(
                            &self.engine,
                            &component.id,
                            channel,
                            msg.get_payload_bytes(),
                        )
                        .await?
                }
            };
        } else {
            log::debug!("No subscription found for {:?}", channel);
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
        engine: &ExecutionContext,
        component: &str,
        channel: &str,
        payload: &[u8],
    ) -> Result<()>;
}

#[cfg(test)]
mod tests;
