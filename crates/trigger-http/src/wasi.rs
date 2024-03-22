use std::net::SocketAddr;

use anyhow::{anyhow, Context, Result};
use futures::TryFutureExt;
use http::{HeaderName, HeaderValue};
use hyper::{Request, Response};
use spin_factor_observe::future::FutureExt;
use spin_factor_outbound_http::wasi_2023_10_18::exports::wasi::http::incoming_handler::Guest as IncomingHandler2023_10_18;
use spin_factor_outbound_http::wasi_2023_11_10::exports::wasi::http::incoming_handler::Guest as IncomingHandler2023_11_10;
use spin_http::routes::RouteMatch;
use tokio::{sync::oneshot, task};
use tracing::{instrument, Instrument, Level};
use wasmtime_wasi_http::{body::HyperIncomingBody as Body, proxy::Proxy, WasiHttpView};

use crate::{
    headers::prepare_request_headers,
    server::{HandlerType, HttpExecutor, WASI_HTTP_EXPORT_2023_10_18, WASI_HTTP_EXPORT_2023_11_10},
    TriggerInstanceBuilder,
};

/// An [`HttpExecutor`] that uses the `wasi:http/incoming-handler` interface.
#[derive(Clone)]
pub struct WasiHttpExecutor {
    pub handler_type: HandlerType,
}

impl HttpExecutor for WasiHttpExecutor {
    #[instrument(name = "spin_trigger_http.execute_wasm", skip_all, err(level = Level::INFO), fields(otel.name = format!("execute_wasm_component {}", route_match.component_id())))]
    async fn execute(
        &self,
        instance_builder: TriggerInstanceBuilder<'_>,
        route_match: &RouteMatch,
        mut req: Request<Body>,
        client_addr: SocketAddr,
    ) -> Result<Response<Body>> {
        let component_id = route_match.component_id();

        tracing::trace!("Executing request using the Wasi executor for component {component_id}");

        let (instance, mut store) = instance_builder.instantiate(()).await?;

        let headers = prepare_request_headers(&req, route_match, client_addr)?;
        req.headers_mut().clear();
        req.headers_mut()
            .extend(headers.into_iter().filter_map(|(n, v)| {
                let Ok(name) = n.parse::<HeaderName>() else {
                    return None;
                };
                let Ok(value) = HeaderValue::from_bytes(v.as_bytes()) else {
                    return None;
                };
                Some((name, value))
            }));

        let mut wasi_http = spin_factor_outbound_http::OutboundHttpFactor::get_wasi_http_impl(
            store.data_mut().factors_instance_state(),
        )
        .context("missing OutboundHttpFactor")?;

        let request = wasi_http.new_incoming_request(req)?;

        let (response_tx, response_rx) = oneshot::channel();
        let response = wasi_http.new_response_outparam(response_tx)?;

        drop(wasi_http);

        enum Handler {
            Latest(Proxy),
            Handler2023_11_10(IncomingHandler2023_11_10),
            Handler2023_10_18(IncomingHandler2023_10_18),
        }

        let handler =
            {
                let mut exports = instance.exports(&mut store);
                match self.handler_type {
                    HandlerType::Wasi2023_10_18 => {
                        let mut instance = exports
                            .instance(WASI_HTTP_EXPORT_2023_10_18)
                            .ok_or_else(|| {
                                anyhow!("export of `{WASI_HTTP_EXPORT_2023_10_18}` not an instance")
                            })?;
                        Handler::Handler2023_10_18(IncomingHandler2023_10_18::new(&mut instance)?)
                    }
                    HandlerType::Wasi2023_11_10 => {
                        let mut instance = exports
                            .instance(WASI_HTTP_EXPORT_2023_11_10)
                            .ok_or_else(|| {
                                anyhow!("export of `{WASI_HTTP_EXPORT_2023_11_10}` not an instance")
                            })?;
                        Handler::Handler2023_11_10(IncomingHandler2023_11_10::new(&mut instance)?)
                    }
                    HandlerType::Wasi0_2 => {
                        drop(exports);
                        Handler::Latest(Proxy::new(&mut store, &instance)?)
                    }
                    HandlerType::Spin => unreachable!("should have used SpinHttpExecutor"),
                    HandlerType::Wagi => unreachable!("should have used WagiExecutor instead"),
                }
            };

        let observe_context = store
            .data_mut()
            .factors_instance_state()
            .observe
            .get_observe_context();

        let span = tracing::debug_span!("execute_wasi");
        let handle = task::spawn(
            async move {
                let result = match handler {
                    Handler::Latest(proxy) => {
                        proxy
                            .wasi_http_incoming_handler()
                            .call_handle(&mut store, request, response)
                            .manage_wasi_observe_spans(observe_context)
                            .instrument(span)
                            .await
                    }
                    Handler::Handler2023_10_18(handler) => {
                        handler
                            .call_handle(&mut store, request, response)
                            .manage_wasi_observe_spans(observe_context)
                            .instrument(span)
                            .await
                    }
                    Handler::Handler2023_11_10(handler) => {
                        handler
                            .call_handle(&mut store, request, response)
                            .manage_wasi_observe_spans(observe_context)
                            .instrument(span)
                            .await
                    }
                };

                tracing::trace!(
                    "wasi-http memory consumed: {}",
                    store.data().core_state().memory_consumed()
                );

                result
            }
            .in_current_span(),
        );

        match response_rx.await {
            Ok(response) => {
                task::spawn(
                    async move {
                        handle
                            .await
                            .context("guest invocation panicked")?
                            .context("guest invocation failed")?;

                        Ok(())
                    }
                    .map_err(|e: anyhow::Error| {
                        tracing::warn!("component error after response: {e:?}");
                    }),
                );

                Ok(response.context("guest failed to produce a response")?)
            }

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
