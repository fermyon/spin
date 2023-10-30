use std::{net::SocketAddr, str, str::FromStr};

use crate::{Body, HttpExecutor, HttpTrigger, Store};
use anyhow::bail;
use anyhow::{anyhow, Context, Result};
use http::{HeaderName, HeaderValue};
use http_body_util::BodyExt;
use hyper::{Request, Response};
use outbound_http::OutboundHttpComponent;
use spin_core::async_trait;
use spin_core::Instance;
use spin_http::body;
use spin_trigger::{EitherInstance, TriggerAppEngine};
use spin_world::v1::http_types;
use std::sync::Arc;
use tokio::{sync::oneshot, task};
use wasmtime_wasi_http::{proxy::Proxy, WasiHttpView};

#[derive(Clone)]
pub struct HttpHandlerExecutor;

#[async_trait]
impl HttpExecutor for HttpHandlerExecutor {
    async fn execute(
        &self,
        engine: &TriggerAppEngine<HttpTrigger>,
        component_id: &str,
        base: &str,
        raw_route: &str,
        req: Request<Body>,
        client_addr: SocketAddr,
    ) -> Result<Response<Body>> {
        tracing::trace!(
            "Executing request using the Spin executor for component {}",
            component_id
        );

        let (instance, mut store) = engine.prepare_instance(component_id).await?;
        let EitherInstance::Component(instance) = instance else {
            unreachable!()
        };

        set_http_origin_from_request(&mut store, engine, &req);

        let resp = match HandlerType::from_exports(instance.exports(&mut store)) {
            Some(HandlerType::Wasi) => Self::execute_wasi(store, instance, base, raw_route, req, client_addr).await?,
            Some(HandlerType::Spin) => {
                Self::execute_spin(store, instance, base, raw_route, req, client_addr)
                    .await
                    .map_err(contextualise_err)?
            }
            None => bail!("Expected component to either export `{}` or `fermyon:spin/inbound-http` but it exported neither", WASI_HTTP_EXPORT)
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
        raw_route: &str,
        req: Request<Body>,
        client_addr: SocketAddr,
    ) -> Result<Response<Body>> {
        let headers = Self::headers(&req, raw_route, base, client_addr)?;
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
        base: &str,
        raw_route: &str,
        mut req: Request<Body>,
        client_addr: SocketAddr,
    ) -> anyhow::Result<Response<Body>> {
        let headers = Self::headers(&req, raw_route, base, client_addr)?;
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

        let proxy = Proxy::new(&mut store, &instance)?;

        let handle = task::spawn(async move {
            let result = proxy
                .wasi_http_incoming_handler()
                .call_handle(&mut store, request, response)
                .await;

            tracing::trace!(
                "wasi-http memory consumed: {}",
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

    fn headers(
        req: &Request<Body>,
        raw: &str,
        base: &str,
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
        for (keys, val) in crate::compute_default_headers(req.uri(), raw, base, host, client_addr)?
        {
            res.push((Self::prepare_header_key(keys[0]), val));
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
enum HandlerType {
    Spin,
    Wasi,
}

const WASI_HTTP_EXPORT: &str = "wasi:http/incoming-handler@0.2.0-rc-2023-10-18";

impl HandlerType {
    /// Determine the handler type from the exports
    fn from_exports(mut exports: wasmtime::component::Exports<'_>) -> Option<HandlerType> {
        if exports.instance(WASI_HTTP_EXPORT).is_some() {
            return Some(HandlerType::Wasi);
        }
        if exports.instance("fermyon:spin/inbound-http").is_some() {
            return Some(HandlerType::Spin);
        }
        None
    }
}

fn set_http_origin_from_request(
    store: &mut Store,
    engine: &TriggerAppEngine<HttpTrigger>,
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

                outbound_http_data.origin = origin.clone();
                let allowed_http_hosts = outbound_http_data.allowed_http_hosts.clone();
                let allowed_hosts = outbound_http_data.allowed_hosts.clone();
                let data = store.as_mut().data_mut().as_mut();
                data.allowed_http_hosts = allowed_http_hosts;
                data.allowed_hosts = allowed_hosts;
            }
            store.as_mut().data_mut().as_mut().origin = Some(origin);
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
