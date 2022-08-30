use anyhow::{anyhow, Result};
use async_trait::async_trait;
use spin_core::Instance;
use spin_trigger::TriggerAppEngine;

use crate::{spin_redis::SpinRedis, RedisExecutor, RedisTrigger, Store};

#[derive(Clone)]
pub struct SpinRedisExecutor;

#[async_trait]
impl RedisExecutor for SpinRedisExecutor {
    async fn execute(
        &self,
        app_engine: &TriggerAppEngine<RedisTrigger>,
        component_id: &str,
        channel: &str,
        payload: &[u8],
    ) -> Result<()> {
        let (instance, store) = app_engine.prepare_instance(component_id).await?;

        match Self::execute_impl(store, instance, channel, payload.to_vec()).await {
            Ok(()) => {
                log::trace!("Request finished OK");
                Ok(())
            }
            Err(e) => {
                log::trace!("Request finished with error {}", e);
                Err(e)
            }
        }
    }
}

impl SpinRedisExecutor {
    pub async fn execute_impl(
        mut store: Store,
        instance: Instance,
        _channel: &str,
        payload: Vec<u8>,
    ) -> Result<()> {
        let engine = SpinRedis::new(&mut store, &instance, |host| host.as_mut())?;

        engine
            .handle_redis_message(&mut store, &payload)
            .await?
            .map_err(|err| anyhow!("{err:?}"))
    }
}
