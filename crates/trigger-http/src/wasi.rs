use crate::{Body, HttpExecutor, HttpTrigger};
use anyhow::{anyhow, Context, Result};
use hyper::{Request, Response};
use spin_core::async_trait;
use spin_trigger::{EitherInstance, TriggerAppEngine};
use std::{net::SocketAddr, str};
use tokio::{sync::oneshot, task};
use wasmtime_wasi_http::{proxy::Proxy, WasiHttpView};

#[derive(Clone)]
pub struct WasiHttpExecutor;

#[async_trait]
impl HttpExecutor for WasiHttpExecutor {
    async fn execute(
        &self,
        engine: &TriggerAppEngine<HttpTrigger>,
        component_id: &str,
        _base: &str,
        _raw_route: &str,
        req: Request<Body>,
        _client_addr: SocketAddr,
    ) -> Result<Response<Body>> {
        tracing::trace!("Executing request using the WASI executor for component {component_id}",);

        let (instance, mut store) = engine.prepare_instance(component_id).await?;
        let EitherInstance::Component(instance) = instance else {
            unreachable!()
        };

        let request = store.as_mut().data_mut().new_incoming_request(req)?;

        let (response_tx, response_rx) = oneshot::channel();
        let response = store
            .as_mut()
            .data_mut()
            .new_response_outparam(response_tx)?;

        let proxy = Proxy::new(&mut store, &instance)?;

        let handle = task::spawn(async move {
            let result = proxy
                .wasi_http_incoming_handler()
                .call_handle(&mut store, request, response)
                .await;

            tracing::trace!("result: {result:?}",);

            tracing::trace!(
                "memory consumed: {}",
                store.as_ref().data().memory_consumed()
            );

            result
        });

        match response_rx.await {
            Ok(response) => Ok(response.context("guest failed to produce a response")?),

            Err(_) => {
                handle
                    .await
                    .context("guest invocation panicked")?
                    .context("guest invocation failed")?;

                Err(anyhow!(
                    "guest failed to produce a response prior to returning"
                ))
            }
        }
    }
}
