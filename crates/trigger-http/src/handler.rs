use std::{net::SocketAddr, str, str::FromStr};

use crate::{Body, ChainedRequestHandler, HttpExecutor, HttpInstance, HttpTrigger, Store};
use anyhow::{anyhow, Context, Result};
use futures::TryFutureExt;
use http::{HeaderName, HeaderValue};
use http_body_util::BodyExt;
use hyper::{Request, Response};
use outbound_http::OutboundHttpComponent;
use spin_core::async_trait;
use spin_core::wasi_2023_10_18::exports::wasi::http::incoming_handler::Guest as IncomingHandler2023_10_18;
use spin_core::wasi_2023_11_10::exports::wasi::http::incoming_handler::Guest as IncomingHandler2023_11_10;
use spin_core::{Component, Engine, Instance};
use spin_http::body;
use spin_http::routes::RouteMatch;
use spin_trigger::TriggerAppEngine;
use spin_world::v1::http_types;
use std::sync::Arc;
use tokio::{sync::oneshot, task};
use tracing::{instrument, Instrument, Level};
use wasmtime_wasi_http::{proxy::Proxy, WasiHttpView};

#[derive(Clone)]
pub struct HttpHandlerExecutor;

#[async_trait]
impl HttpExecutor for HttpHandlerExecutor {
    #[instrument(name = "spin_trigger_http.execute_wasm", skip_all, err(level = Level::INFO), fields(otel.name = format!("execute_wasm_component {}", route_match.component_id())))]
    async fn execute(
        &self,
        engine: Arc<TriggerAppEngine<HttpTrigger>>,
        base: &str,
        route_match: &RouteMatch,
        req: Request<Body>,
        client_addr: SocketAddr,
    ) -> Result<Response<Body>> {
        let component_id = route_match.component_id();

        tracing::trace!(
            "Executing request using the Spin executor for component {}",
            component_id
        );

        let (instance, mut store) = engine.prepare_instance(component_id).await?;
        let HttpInstance::Component(instance, ty) = instance else {
            unreachable!()
        };

        set_http_origin_from_request(&mut store, engine.clone(), self, &req);

        // set the client tls options for the current component_id.
        // The OutboundWasiHttpHandler in this file is only used
        // when making http-request from a http-trigger component.
        // The outbound http requests from other triggers such as Redis
        // uses OutboundWasiHttpHandler defined in spin_core crate.
        store.as_mut().data_mut().as_mut().client_tls_opts =
            engine.get_client_tls_opts(component_id);

        let resp = match ty {
            HandlerType::Spin => {
                Self::execute_spin(store, instance, base, route_match, req, client_addr)
                    .await
                    .map_err(contextualise_err)?
            }
            _ => {
                Self::execute_wasi(store, instance, ty, base, route_match, req, client_addr).await?
            }
        };

        tracing::info!(
            "Request finished, sending response with status code {}",
            resp.status()
        );
        Ok(resp)
    }
}

impl HttpHandlerExecutor {
    pub async fn execute_spin(
        mut store: Store,
        instance: Instance,
        base: &str,
        route_match: &RouteMatch,
        req: Request<Body>,
        client_addr: SocketAddr,
    ) -> Result<Response<Body>> {
        let headers = Self::headers(&req, base, route_match, client_addr)?;
        let func = instance
            .exports(&mut store)
            .instance("fermyon:spin/inbound-http")
            // Safe since we have already checked that this instance exists
            .expect("no fermyon:spin/inbound-http found")
            .typed_func::<(http_types::Request,), (http_types::Response,)>("handle-request")?;

        let (parts, body) = req.into_parts();
        let bytes = body.collect().await?.to_bytes().to_vec();

        let method = if let Some(method) = Self::method(&parts.method) {
            method
        } else {
            return Ok(Response::builder()
                .status(http::StatusCode::METHOD_NOT_ALLOWED)
                .body(body::empty())?);
        };

        // Preparing to remove the params field. We are leaving it in place for now
        // to avoid breaking the ABI, but no longer pass or accept values in it.
        // https://github.com/fermyon/spin/issues/663
        let params = vec![];

        let uri = match parts.uri.path_and_query() {
            Some(u) => u.to_string(),
            None => parts.uri.to_string(),
        };

        let req = http_types::Request {
            method,
            uri,
            headers,
            params,
            body: Some(bytes),
        };

        let (resp,) = func.call_async(&mut store, (req,)).await?;

        if resp.status < 100 || resp.status > 600 {
            tracing::error!("malformed HTTP status code");
            return Ok(Response::builder()
                .status(http::StatusCode::INTERNAL_SERVER_ERROR)
                .body(body::empty())?);
        };

        let mut response = http::Response::builder().status(resp.status);
        if let Some(headers) = response.headers_mut() {
            Self::append_headers(headers, resp.headers)?;
        }

        let body = match resp.body {
            Some(b) => body::full(b.into()),
            None => body::empty(),
        };

        Ok(response.body(body)?)
    }

    fn method(m: &http::Method) -> Option<http_types::Method> {
        Some(match *m {
            http::Method::GET => http_types::Method::Get,
            http::Method::POST => http_types::Method::Post,
            http::Method::PUT => http_types::Method::Put,
            http::Method::DELETE => http_types::Method::Delete,
            http::Method::PATCH => http_types::Method::Patch,
            http::Method::HEAD => http_types::Method::Head,
            http::Method::OPTIONS => http_types::Method::Options,
            _ => return None,
        })
    }

    async fn execute_wasi(
        mut store: Store,
        instance: Instance,
        ty: HandlerType,
        base: &str,
        route_match: &RouteMatch,
        mut req: Request<Body>,
        client_addr: SocketAddr,
    ) -> anyhow::Result<Response<Body>> {
        let headers = Self::headers(&req, base, route_match, client_addr)?;
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
        let request = store.as_mut().data_mut().new_incoming_request(req)?;

        let (response_tx, response_rx) = oneshot::channel();
        let response = store
            .as_mut()
            .data_mut()
            .new_response_outparam(response_tx)?;

        enum Handler {
            Latest(Proxy),
            Handler2023_11_10(IncomingHandler2023_11_10),
            Handler2023_10_18(IncomingHandler2023_10_18),
        }

        let handler =
            {
                let mut exports = instance.exports(&mut store);
                match ty {
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
                    HandlerType::Spin => panic!("should have used execute_spin instead"),
                }
            };

        let span = tracing::debug_span!("execute_wasi");
        let handle = task::spawn(
            async move {
                let result = match handler {
                    Handler::Latest(proxy) => {
                        proxy
                            .wasi_http_incoming_handler()
                            .call_handle(&mut store, request, response)
                            .instrument(span)
                            .await
                    }
                    Handler::Handler2023_10_18(proxy) => {
                        proxy
                            .call_handle(&mut store, request, response)
                            .instrument(span)
                            .await
                    }
                    Handler::Handler2023_11_10(proxy) => {
                        proxy
                            .call_handle(&mut store, request, response)
                            .instrument(span)
                            .await
                    }
                };

                tracing::trace!(
                    "wasi-http memory consumed: {}",
                    store.as_ref().data().memory_consumed()
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

    fn headers(
        req: &Request<Body>,
        base: &str,
        route_match: &RouteMatch,
        client_addr: SocketAddr,
    ) -> Result<Vec<(String, String)>> {
        let mut res = Vec::new();
        for (name, value) in req
            .headers()
            .iter()
            .map(|(name, value)| (name.to_string(), std::str::from_utf8(value.as_bytes())))
        {
            let value = value?.to_string();
            res.push((name, value));
        }

        let default_host = http::HeaderValue::from_str("localhost")?;
        let host = std::str::from_utf8(
            req.headers()
                .get("host")
                .unwrap_or(&default_host)
                .as_bytes(),
        )?;

        // Set the environment information (path info, base path, etc) as headers.
        // In the future, we might want to have this information in a context
        // object as opposed to headers.
        for (keys, val) in
            crate::compute_default_headers(req.uri(), base, host, route_match, client_addr)?
        {
            res.push((Self::prepare_header_key(&keys[0]), val));
        }

        Ok(res)
    }

    fn prepare_header_key(key: &str) -> String {
        key.replace('_', "-").to_ascii_lowercase()
    }

    fn append_headers(res: &mut http::HeaderMap, src: Option<Vec<(String, String)>>) -> Result<()> {
        if let Some(src) = src {
            for (k, v) in src.iter() {
                res.insert(
                    http::header::HeaderName::from_str(k)?,
                    http::header::HeaderValue::from_str(v)?,
                );
            }
        };

        Ok(())
    }
}

/// Whether this handler uses the custom Spin http handler interface for wasi-http
#[derive(Copy, Clone)]
pub enum HandlerType {
    Spin,
    Wasi0_2,
    Wasi2023_11_10,
    Wasi2023_10_18,
}

const WASI_HTTP_EXPORT_2023_10_18: &str = "wasi:http/incoming-handler@0.2.0-rc-2023-10-18";
const WASI_HTTP_EXPORT_2023_11_10: &str = "wasi:http/incoming-handler@0.2.0-rc-2023-11-10";
const WASI_HTTP_EXPORT_0_2_0: &str = "wasi:http/incoming-handler@0.2.0";

impl HandlerType {
    /// Determine the handler type from the exports of a component
    pub fn from_component<T>(engine: &Engine<T>, component: &Component) -> Result<HandlerType> {
        let mut handler_ty = None;

        let mut set = |ty: HandlerType| {
            if handler_ty.is_none() {
                handler_ty = Some(ty);
                Ok(())
            } else {
                Err(anyhow!(
                    "component exports multiple different handlers but \
                     it's expected to export only one"
                ))
            }
        };
        let ty = component.component_type();
        for (name, _) in ty.exports(engine.as_ref()) {
            match name {
                WASI_HTTP_EXPORT_2023_10_18 => set(HandlerType::Wasi2023_10_18)?,
                WASI_HTTP_EXPORT_2023_11_10 => set(HandlerType::Wasi2023_11_10)?,
                WASI_HTTP_EXPORT_0_2_0 => set(HandlerType::Wasi0_2)?,
                "fermyon:spin/inbound-http" => set(HandlerType::Spin)?,
                _ => {}
            }
        }

        handler_ty.ok_or_else(|| {
            anyhow!(
                "Expected component to either export `{WASI_HTTP_EXPORT_2023_10_18}`, \
                 `{WASI_HTTP_EXPORT_2023_11_10}`, `{WASI_HTTP_EXPORT_0_2_0}`, \
                 or `fermyon:spin/inbound-http` but it exported none of those"
            )
        })
    }
}

fn set_http_origin_from_request(
    store: &mut Store,
    engine: Arc<TriggerAppEngine<HttpTrigger>>,
    handler: &HttpHandlerExecutor,
    req: &Request<Body>,
) {
    if let Some(authority) = req.uri().authority() {
        if let Some(scheme) = req.uri().scheme_str() {
            let origin = format!("{}://{}", scheme, authority);
            if let Some(outbound_http_handle) = engine
                .engine
                .find_host_component_handle::<Arc<OutboundHttpComponent>>()
            {
                let outbound_http_data = store
                    .host_components_data()
                    .get_or_insert(outbound_http_handle);

                outbound_http_data.origin.clone_from(&origin);
                store.as_mut().data_mut().as_mut().allowed_hosts =
                    outbound_http_data.allowed_hosts.clone();
            }

            let chained_request_handler = ChainedRequestHandler {
                engine: engine.clone(),
                executor: handler.clone(),
            };
            store.as_mut().data_mut().as_mut().origin = Some(origin);
            store.as_mut().data_mut().as_mut().chained_handler = Some(chained_request_handler);
        }
    }
}

fn contextualise_err(e: anyhow::Error) -> anyhow::Error {
    if e.to_string()
        .contains("failed to find function export `canonical_abi_free`")
    {
        e.context(
            "component is not compatible with Spin executor - should this use the Wagi executor?",
        )
    } else {
        e
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spin_header_keys() {
        assert_eq!(
            HttpHandlerExecutor::prepare_header_key("SPIN_FULL_URL"),
            "spin-full-url".to_string()
        );
        assert_eq!(
            HttpHandlerExecutor::prepare_header_key("SPIN_PATH_INFO"),
            "spin-path-info".to_string()
        );
        assert_eq!(
            HttpHandlerExecutor::prepare_header_key("SPIN_RAW_COMPONENT_ROUTE"),
            "spin-raw-component-route".to_string()
        );
    }
}
