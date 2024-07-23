use std::{net::SocketAddr, str, str::FromStr};

use crate::types::Scheme;
use crate::{
    Body, ChainedRequestHandler, HttpExecutor, HttpInstance, HttpRuntimeData, HttpTrigger, Store,
};
use anyhow::{anyhow, Context, Result};
use futures::TryFutureExt;
use http::{HeaderName, HeaderValue};
use http_body_util::BodyExt;
use hyper::{Request, Response};
use outbound_http::OutboundHttpComponent;
use spin_core::async_trait;
use spin_core::wasi_2023_10_18::exports::wasi::http::incoming_handler::{
    Guest as Guest2023_10_18, GuestPre as GuestPre2023_10_18,
};
use spin_core::wasi_2023_11_10::exports::wasi::http::incoming_handler::{
    Guest as Guest2023_11_10, GuestPre as GuestPre2023_11_10,
};
use spin_core::Instance;
use spin_http::body;
use spin_http::routes::RouteMatch;
use spin_trigger::TriggerAppEngine;
use spin_world::v1::http_types;
use std::sync::Arc;
use tokio::{sync::oneshot, task};
use tracing::{instrument, Instrument, Level};
use wasmtime_wasi_http::{
    bindings::{Proxy, ProxyPre},
    WasiHttpView,
};

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
        let HttpInstance::Component(handler) = instance else {
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

        let resp = match handler {
            Handler::Spin(instance) => {
                Self::execute_spin(store, instance, base, route_match, req, client_addr)
                    .await
                    .map_err(contextualise_err)?
            }
            _ => Self::execute_wasi(store, handler, base, route_match, req, client_addr).await?,
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
        let handle_request = instance
            .get_export(&mut store, None, "fermyon:spin/inbound-http")
            .and_then(|inbound_http| {
                instance.get_export(&mut store, Some(&inbound_http), "handle-request")
            })
            // Safe since we have already checked that this instance exists
            .expect("no fermyon:spin/inbound-http/handle-request found");

        let func = instance.get_typed_func::<(http_types::Request,), (http_types::Response,)>(
            &mut store,
            &handle_request,
        )?;

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
        handler: Handler,
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

        let (parts, body) = req.into_parts();
        let body = wasmtime_wasi_http::body::HostIncomingBody::new(
            body,
            // TODO: this needs to be plumbed through
            std::time::Duration::from_millis(600 * 1000),
        );
        let request = wasmtime_wasi_http::types::HostIncomingRequest::new(
            store.as_mut().data_mut(),
            parts,
            Scheme::Http,
            Some(body),
        )?;
        let request = store.as_mut().data_mut().table().push(request)?;

        let (response_tx, response_rx) = oneshot::channel();
        let response = store
            .as_mut()
            .data_mut()
            .new_response_outparam(response_tx)?;

        let span = tracing::debug_span!("execute_wasi");
        let handle = task::spawn(
            async move {
                let result = match handler {
                    Handler::Wasi0_2(proxy) => {
                        proxy
                            .wasi_http_incoming_handler()
                            .call_handle(&mut store, request, response)
                            .instrument(span)
                            .await
                    }
                    Handler::Wasi2023_11_10(proxy) => {
                        proxy
                            .call_handle(&mut store, request, response)
                            .instrument(span)
                            .await
                    }
                    Handler::Wasi2023_10_18(proxy) => {
                        proxy
                            .call_handle(&mut store, request, response)
                            .instrument(span)
                            .await
                    }
                    Handler::Spin(_) => unreachable!("should be in `execute_spin` instead"),
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

/// Pre-instantiated version of the kinds of handlers that this trigger
/// supports.
#[derive(Clone)]
pub enum HandlerPre {
    Spin(spin_core::InstancePre<HttpRuntimeData>),
    Wasi0_2(ProxyPre<spin_core::Data<HttpRuntimeData>>),
    Wasi2023_11_10(spin_core::InstancePre<HttpRuntimeData>, GuestPre2023_11_10),
    Wasi2023_10_18(spin_core::InstancePre<HttpRuntimeData>, GuestPre2023_10_18),
}

/// Instantiated version of [`HandlerPre`] for the types of components this
/// trigger supports.
pub enum Handler {
    Spin(spin_core::Instance),
    Wasi0_2(Proxy),
    Wasi2023_11_10(Guest2023_11_10),
    Wasi2023_10_18(Guest2023_10_18),
}

const WASI_HTTP_EXPORT_2023_10_18: &str = "wasi:http/incoming-handler@0.2.0-rc-2023-10-18";
const WASI_HTTP_EXPORT_2023_11_10: &str = "wasi:http/incoming-handler@0.2.0-rc-2023-11-10";
const WASI_HTTP_EXPORT_0_2_0: &str = "wasi:http/incoming-handler@0.2.0";

impl HandlerPre {
    /// Determine the handler type from the exports of a component
    pub fn from_instance_pre(pre: spin_core::InstancePre<HttpRuntimeData>) -> Result<HandlerPre> {
        let mut handler = None;

        let mut set = |pre: HandlerPre| {
            if handler.is_none() {
                handler = Some(pre);
                Ok(())
            } else {
                Err(anyhow!(
                    "component exports multiple different handlers but \
                     it's expected to export only one"
                ))
            }
        };
        if let Ok(guest) = GuestPre2023_10_18::new(pre.component()) {
            set(HandlerPre::Wasi2023_10_18(pre.clone(), guest))?;
        }
        if let Ok(guest) = GuestPre2023_11_10::new(pre.component()) {
            set(HandlerPre::Wasi2023_11_10(pre.clone(), guest))?;
        }
        if let Ok(pre) = ProxyPre::new(pre.as_ref().clone()) {
            set(HandlerPre::Wasi0_2(pre))?;
        }
        if pre
            .component()
            .export_index(None, "fermyon:spin/inbound-http")
            .is_some()
        {
            set(HandlerPre::Spin(pre))?;
        }

        handler.ok_or_else(|| {
            anyhow!(
                "Expected component to either export `{WASI_HTTP_EXPORT_2023_10_18}`, \
                 `{WASI_HTTP_EXPORT_2023_11_10}`, `{WASI_HTTP_EXPORT_0_2_0}`, \
                 or `fermyon:spin/inbound-http` but it exported none of those"
            )
        })
    }

    pub async fn instantiate(&self, store: &mut Store) -> Result<Handler> {
        match self {
            HandlerPre::Spin(pre) => Ok(Handler::Spin(pre.instantiate_async(store).await?)),
            HandlerPre::Wasi0_2(pre) => Ok(Handler::Wasi0_2(pre.instantiate_async(store).await?)),
            HandlerPre::Wasi2023_11_10(pre, guest) => {
                let instance = pre.instantiate_async(store).await?;
                Ok(Handler::Wasi2023_11_10(guest.load(store, &instance)?))
            }
            HandlerPre::Wasi2023_10_18(pre, guest) => {
                let instance = pre.instantiate_async(store).await?;
                Ok(Handler::Wasi2023_10_18(guest.load(store, &instance)?))
            }
        }
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
