//! Implementation for the Spin Redis engine.

mod spin;

use crate::spin::SpinRedisExecutor;
use anyhow::{ensure, Result};
use async_trait::async_trait;
use futures::StreamExt;
use redis::Client;
use spin_config::{Configuration, CoreComponent, RedisConfig};
use spin_engine::{Builder, ExecutionContextConfiguration};
use spin_redis::SpinRedisData;
use std::{collections::HashMap, path::PathBuf, sync::Arc};

wit_bindgen_wasmtime::import!("../../wit/ephemeral/spin-redis.wit");

type ExecutionContext = spin_engine::ExecutionContext<SpinRedisData>;
type RuntimeContext = spin_engine::RuntimeContext<SpinRedisData>;

/// The Spin Redis trigger.
#[derive(Clone)]
pub struct RedisTrigger {
    /// Configuration for the application.
    app: Configuration<CoreComponent>,
    /// Spin execution context.
    engine: Arc<ExecutionContext>,
    /// Map from channel name to tuple of component name & index.
    subscriptions: HashMap<String, usize>,
}

impl RedisTrigger {
    /// Create a new Spin Redis trigger.
    pub async fn new(
        app: Configuration<CoreComponent>,
        wasmtime: Option<wasmtime::Config>,
        log_dir: Option<PathBuf>,
    ) -> Result<Self> {
        ensure!(
            app.info.trigger.as_redis().is_some(),
            "Application trigger is not Redis"
        );

        let mut config = ExecutionContextConfiguration::new(app.clone(), log_dir);
        if let Some(wasmtime) = wasmtime {
            config.wasmtime = wasmtime;
        };
        let engine = Arc::new(Builder::build_default(config).await?);
        log::debug!("Created new Redis trigger.");

        let subscriptions =
            app.components
                .iter()
                .enumerate()
                .fold(HashMap::new(), |mut map, (idx, c)| {
                    if let Some(RedisConfig { channel, .. }) = c.trigger.as_redis() {
                        map.insert(channel.clone(), idx);
                    }
                    map
                });

        Ok(Self {
            app,
            engine,
            subscriptions,
        })
    }

    /// Run the Redis trigger indefinitely.
    pub async fn run(&self) -> Result<()> {
        // We can unwrap here because the trigger type has already been asserted in `RedisTrigger::new`
        let address = self.app.info.trigger.as_redis().cloned().unwrap().address;

        log::info!("Connecting to Redis server at {}", address);
        let client = Client::open(address.clone())?;
        let mut pubsub = client.get_async_connection().await?.into_pubsub();

        // Subscribe to channels
        for (subscription, id) in self.subscriptions.iter() {
            let name = &self.app.components[*id].id;
            log::info!(
                "Subscribed component #{} ({}) to channel: {}",
                id,
                name,
                subscription
            );
            pubsub.subscribe(subscription).await?;
        }

        let mut stream = pubsub.on_message();
        loop {
            match stream.next().await {
                Some(msg) => drop(self.handle(msg).await),
                None => log::debug!("Empty message"),
            };
        }
    }

    // Handle the message.
    async fn handle(&self, msg: redis::Msg) -> Result<()> {
        let channel = msg.get_channel_name();
        log::info!("Received message on channel {:?}", channel);

        if let Some(id) = self.subscriptions.get(channel).copied() {
            let component = &self.app.components[id];
            let executor = component
                .trigger
                .as_redis()
                .cloned()
                .unwrap() // TODO
                .executor
                .unwrap_or_default();

            match executor {
                spin_config::RedisExecutor::Spin => {
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
