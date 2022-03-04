use crate::{
    spin_redis_trigger::SpinRedisTrigger, ExecutionContext, RedisExecutor, RuntimeContext,
};
use anyhow::Result;
use async_trait::async_trait;
use tokio::task::spawn_blocking;
use wasmtime::{Instance, Store};

#[derive(Clone)]
pub struct SpinRedisExecutor;

#[async_trait]
impl RedisExecutor for SpinRedisExecutor {
    async fn execute(
        &self,
        engine: &ExecutionContext,
        component: &str,
        channel: &str,
        payload: &[u8],
    ) -> Result<()> {
        log::trace!(
            "Executing request using the Spin executor for component {}",
            component
        );
        let (store, instance) = engine.prepare_component(component, None, None, None, None)?;

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
        mut store: Store<RuntimeContext>,
        instance: Instance,
        _channel: &str,
        payload: Vec<u8>,
    ) -> Result<()> {
        let engine =
            SpinRedisTrigger::new(&mut store, &instance, |host| host.data.as_mut().unwrap())?;

        let _res = spawn_blocking(move || -> Result<crate::spin_redis_trigger::Error> {
            match engine.handler(&mut store, &payload) {
                Ok(_) => Ok(crate::spin_redis_trigger::Error::Success),
                Err(_) => Ok(crate::spin_redis_trigger::Error::Error),
            }
        })
        .await??;

        Ok(())
    }
}
