use crate::{MqttExecutor, MqttTrigger, Store};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use spin_core::Instance;
use spin_trigger::{EitherInstance, TriggerAppEngine};
use spin_world::mqtt_types::{Error, Payload};

#[derive(Clone)]
pub struct SpinMqttExecutor;

#[async_trait]
impl MqttExecutor for SpinMqttExecutor {
    async fn execute(
        &self,
        engine: &TriggerAppEngine<MqttTrigger>,
        component_id: &str,
        topic: &str,
        payload: &[u8],
    ) -> Result<()> {
        tracing::trace!("Executing request using the Spin executor for component {component_id}");

        let (instance, store) = engine.prepare_instance(component_id).await?;
        let EitherInstance::Component(instance) = instance else {
            unreachable!()
        };

        match Self::execute_impl(store, instance, topic, payload.to_vec()).await {
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

impl SpinMqttExecutor {
    pub async fn execute_impl(
        mut store: Store,
        instance: Instance,
        _topic: &str,
        payload: Vec<u8>,
    ) -> Result<()> {
        let func = instance
            .exports(&mut store)
            .instance("fermyon:spin/inbound-mqtt")
            .ok_or_else(|| anyhow!("no fermyon:spin/inbound-mqtt instance found"))?
            .typed_func::<(Payload,), (Result<(), Error>,)>("handle-message")?;

        match func.call_async(store, (payload,)).await? {
            (Ok(()) | Err(Error::Success),) => Ok(()),
            _ => Err(anyhow!("`handle-message` returned an error")),
        }
    }
}
