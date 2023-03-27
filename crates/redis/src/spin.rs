use anyhow::{anyhow, Result};
use async_trait::async_trait;
use spin_core::{
    redis_types::{Error, PayloadParam},
    Instance,
};
use spin_trigger::{EitherInstance, TriggerAppEngine};

use crate::{RedisExecutor, RedisTrigger, Store};

#[derive(Clone)]
pub struct SpinRedisExecutor;

#[async_trait]
impl RedisExecutor for SpinRedisExecutor {
    async fn execute(
        &self,
        engine: &TriggerAppEngine<RedisTrigger>,
        component_id: &str,
        channel: &str,
        payload: &[u8],
    ) -> Result<()> {
        tracing::trace!("Executing request using the Spin executor for component {component_id}");

        let (instance, store) = engine.prepare_instance(component_id).await?;
        let EitherInstance::Component(instance) = instance else {
            unreachable!()
        };

        match Self::execute_impl(store, instance, channel, payload.to_vec()).await {
            Ok(()) => {
                tracing::trace!("Request finished OK");
                Ok(())
            }
            Err(e) => {
                tracing::trace!("Request finished with error {e}");
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
        let func = instance
            .exports(&mut store)
            .instance("inbound-redis")
            .ok_or_else(|| anyhow!("no inbound-redis instance found"))?
            .typed_func::<(PayloadParam,), (Result<(), Error>,)>("handle-message")?;

        match func.call_async(store, (&payload,)).await? {
            (Ok(()) | Err(Error::Success),) => Ok(()),
            _ => Err(anyhow!("`handle-message` returned an error")),
        }
    }
}
