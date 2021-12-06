use anyhow::Result;
use async_trait::async_trait;
use futures::StreamExt;
use redis::Client;
use spin_redis_trigger_v01::*;
use std::{sync::Arc, time::Instant};
use wasmtime::{Instance, Store};

wai_bindgen_wasmtime::import!("crates/redis/wit/spin_redis_trigger_v01.wit");

type ExecutionContext = spin_engine::ExecutionContext<SpinRedisTriggerV01Data>;
type RuntimeContext = spin_engine::RuntimeContext<SpinRedisTriggerV01Data>;

#[derive(Clone)]
pub struct RedisEngine(pub Arc<ExecutionContext>);

#[async_trait]
impl Redis for RedisEngine {
    async fn execute(&self, payload: &[u8]) -> Result<()> {
        let start = Instant::now();

        let (store, instance) = self.0.prepare(None)?;
        self.execute_impl(store, instance, payload)?;
        log::info!("Request execution time: {:#?}", start.elapsed());

        Ok(())
    }
}

impl RedisEngine {
    pub fn execute_impl(
        &self,
        mut store: Store<RuntimeContext>,
        instance: Instance,
        payload: &[u8],
    ) -> Result<()> {
        let r =
            SpinRedisTriggerV01::new(&mut store, &instance, |host| host.data.as_mut().unwrap())?;

        let _ = r.handler(&mut store, payload)?;
        Ok(())
    }
}

#[async_trait]
pub trait Redis {
    async fn execute(&self, payload: &[u8]) -> Result<()>;
}

pub struct RedisTrigger {
    pub address: String,
    pub channel: String,
}

impl RedisTrigger {
    pub async fn run(&self, runtime: impl Redis) -> Result<()> {
        let addr = &self.address.clone();
        let ch = &self.channel.clone();

        let client = Client::open(addr.as_str())?;
        let mut pubsub = client.get_async_connection().await?.into_pubsub();
        pubsub.subscribe(ch).await?;
        println!("Subscribed to channel: {}", ch);
        let mut stream = pubsub.on_message();
        loop {
            match stream.next().await {
                Some(p) => {
                    let payload = p.get_payload_bytes();
                    runtime.execute(payload).await?;
                }
                None => log::debug!("Empty message"),
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{RedisEngine, RedisTrigger};
    use spin_engine::{Config, ExecutionContextBuilder};
    use std::sync::Arc;

    const RUST_ENTRYPOINT_PATH: &str = "tests/rust/target/wasm32-wasi/release/rust.wasm";

    #[tokio::test]
    #[allow(unused)]
    async fn test_pubsub() {
        let trigger = RedisTrigger {
            address: "redis://localhost:6379".to_string(),
            channel: "channel".to_string(),
        };

        let engine =
            ExecutionContextBuilder::build_default(RUST_ENTRYPOINT_PATH, Config::default())
                .unwrap();
        let engine = RedisEngine(Arc::new(engine));

        trigger.run(engine).await.unwrap();
    }
}
