use std::{net::SocketAddr, str, str::FromStr};

use anyhow::Result;
use async_trait::async_trait;
use hyper::{Body, Request, Response};
use spin_core::Instance;
use spin_trigger::TriggerAppEngine;
use tracing::log;

use crate::{
    spin_http::{Method, SpinHttp},
    HttpExecutor, HttpTrigger, Store,
};

#[derive(Clone)]
pub struct SpinHttpExecutor;

#[async_trait]
impl HttpExecutor for SpinHttpExecutor {
    async fn execute(
        &self,
        app_engine: &TriggerAppEngine<HttpTrigger>,
        component_id: &str,
        raw_route: &str,
        req: Request<Body>,
        _client_addr: SocketAddr,
    ) -> Result<Response<Body>> {
        log::trace!(
            "Executing request using the Spin executor for component {}",
            component_id
        );

        let (instance, store) = app_engine.prepare_instance(component_id).await?;

        let resp = Self::execute_impl(store, instance, raw_route, req)
            .await
            .map_err(contextualise_err)?;

        log::info!(
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
        raw_route: &str,
        req: Request<Body>,
    ) -> Result<Response<Body>> {
        let headers;
        let mut req = req;
        {
            headers = Self::headers(&mut req, raw_route)?;
        }

        let engine = SpinHttp::new(&mut store, &instance, |host| host.as_mut())?;
        let (parts, bytes) = req.into_parts();
        let bytes = hyper::body::to_bytes(bytes).await?.to_vec();

        let method = Self::method(&parts.method);

        let headers: Vec<(&str, &str)> = headers
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        // Preparing to remove the params field. We are leaving it in place for now
        // to avoid breaking the ABI, but no longer pass or accept values in it.
        // https://github.com/fermyon/spin/issues/663
        let params = vec![];

        let body = Some(&bytes[..]);
        let uri = match parts.uri.path_and_query() {
            Some(u) => u.to_string(),
            None => parts.uri.to_string(),
        };

        let req = crate::spin_http::Request {
            method,
            uri: &uri,
            headers: &headers,
            params: &params,
            body,
        };

        let res = engine.handle_http_request(&mut store, req).await?;

        if res.status < 100 || res.status > 600 {
            log::error!("malformed HTTP status code");
            return Ok(Response::builder()
                .status(http::StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::empty())?);
        };

        let mut response = http::Response::builder().status(res.status);
        if let Some(headers) = response.headers_mut() {
            Self::append_headers(headers, res.headers)?;
        }

        let body = match res.body {
            Some(b) => Body::from(b),
            None => Body::empty(),
        };

        Ok(response.body(body)?)
    }

    fn method(m: &http::Method) -> Method {
        match *m {
            http::Method::GET => Method::Get,
            http::Method::POST => Method::Post,
            http::Method::PUT => Method::Put,
            http::Method::DELETE => Method::Delete,
            http::Method::PATCH => Method::Patch,
            http::Method::HEAD => Method::Head,
            http::Method::OPTIONS => Method::Options,
            _ => todo!(),
        }
    }

    fn headers(req: &mut Request<Body>, raw: &str) -> Result<Vec<(String, String)>> {
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

        // Set the environment information (path info, etc) as headers.
        // In the future, we might want to have this information in a context
        // object as opposed to headers.
        for (keys, val) in crate::compute_default_headers(req.uri(), raw, host)? {
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
