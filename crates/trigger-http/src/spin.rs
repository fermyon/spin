use std::{net::SocketAddr, str, str::FromStr};

use crate::{HttpExecutor, HttpTrigger, Store};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use hyper::{Body, Request, Response};
use outbound_http::OutboundHttpComponent;
use spin_core::Instance;
use spin_trigger::{EitherInstance, TriggerAppEngine};
use spin_world::http_types;
use std::sync::Arc;

#[derive(Clone)]
pub struct SpinHttpExecutor;

#[async_trait]
impl HttpExecutor for SpinHttpExecutor {
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

        let resp = Self::execute_impl(store, instance, base, raw_route, req, client_addr)
            .await
            .map_err(contextualise_err)?;

        tracing::info!(
            "Request finished, sending response with status code {}",
            resp.status()
        );
        Ok(resp)
    }
}

impl SpinHttpExecutor {
    pub async fn execute_impl(
        mut store: Store,
        instance: Instance,
        base: &str,
        raw_route: &str,
        req: Request<Body>,
        client_addr: SocketAddr,
    ) -> Result<Response<Body>> {
        let headers;
        let mut req = req;
        {
            headers = Self::headers(&mut req, raw_route, base, client_addr)?;
        }

        let func = instance
            .exports(&mut store)
            .instance("fermyon:spin/inbound-http")
            .ok_or_else(|| anyhow!("no fermyon:spin/inbound-http instance found"))?
            .typed_func::<(http_types::Request,), (http_types::Response,)>("handle-request")?;

        let (parts, bytes) = req.into_parts();
        let bytes = hyper::body::to_bytes(bytes).await?.to_vec();

        let method = if let Some(method) = Self::method(&parts.method) {
            method
        } else {
            return Ok(Response::builder()
                .status(http::StatusCode::METHOD_NOT_ALLOWED)
                .body(Body::empty())?);
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
                .body(Body::empty())?);
        };

        let mut response = http::Response::builder().status(resp.status);
        if let Some(headers) = response.headers_mut() {
            Self::append_headers(headers, resp.headers)?;
        }

        let body = match resp.body {
            Some(b) => Body::from(b),
            None => Body::empty(),
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

    fn headers(
        req: &mut Request<Body>,
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

fn set_http_origin_from_request(
    store: &mut Store,
    engine: &TriggerAppEngine<HttpTrigger>,
    req: &Request<Body>,
) {
    if let Some(authority) = req.uri().authority() {
        if let Some(scheme) = req.uri().scheme_str() {
            if let Some(outbound_http_handle) = engine
                .engine
                .find_host_component_handle::<Arc<OutboundHttpComponent>>()
            {
                let outbound_http_data = store
                    .host_components_data()
                    .get_or_insert(outbound_http_handle);

                outbound_http_data.origin = format!("{}://{}", scheme, authority);
            }
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
            SpinHttpExecutor::prepare_header_key("SPIN_FULL_URL"),
            "spin-full-url".to_string()
        );
        assert_eq!(
            SpinHttpExecutor::prepare_header_key("SPIN_PATH_INFO"),
            "spin-path-info".to_string()
        );
        assert_eq!(
            SpinHttpExecutor::prepare_header_key("SPIN_RAW_COMPONENT_ROUTE"),
            "spin-raw-component-route".to_string()
        );
    }
}
