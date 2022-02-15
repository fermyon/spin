use crate::routes::RoutePattern;
use crate::spin_http::{Method, SpinHttp};
use crate::HttpExecutor;
use crate::{ExecutionContext, RuntimeContext};
use anyhow::Result;
use async_trait::async_trait;
use http::Uri;
use hyper::{Body, Request, Response};
use std::{net::SocketAddr, str::FromStr};
use tracing::log;
use wasmtime::{Instance, Store};

#[derive(Clone)]
pub struct SpinHttpExecutor;

#[async_trait]
impl HttpExecutor for SpinHttpExecutor {
    async fn execute(
        engine: &ExecutionContext,
        component: &str,
        raw_route: &str,
        req: Request<Body>,
        _client_addr: SocketAddr,
    ) -> Result<Response<Body>> {
        log::trace!(
            "Executing request using the Spin executor for component {}",
            component
        );
        let (store, instance) = engine.prepare_component(component, None, None, None)?;
        let res = Self::execute_impl(store, instance, raw_route, req).await?;
        log::info!(
            "Request finished, sending response with status code {}",
            res.status()
        );
        Ok(res)
    }
}

impl SpinHttpExecutor {
    pub async fn execute_impl(
        mut store: Store<RuntimeContext>,
        instance: Instance,
        raw_route: &str,
        req: Request<Body>,
    ) -> Result<Response<Body>> {
        let engine = SpinHttp::new(&mut store, &instance, |host| host.data.as_mut().unwrap())?;
        let (parts, bytes) = req.into_parts();
        let bytes = hyper::body::to_bytes(bytes).await?.to_vec();
        let body = Some(&bytes[..]);

        let method = Self::method(&parts.method);
        let headers = &Self::headers(&parts.headers)?;
        let params = &Self::params(&parts.uri)?;
        let params: Vec<(&str, &str)> = params
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        let req = crate::spin_http::Request {
            method,
            uri: &RoutePattern::from(raw_route).relative(&parts.uri.to_string()),
            headers,
            params: &params,
            body,
        };

        let res = engine.handler(&mut store, req)?;
        let mut response = http::Response::builder().status(res.status);
        Self::append_headers(response.headers_mut().unwrap(), res.headers)?;

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
            _ => todo!(),
        }
    }

    fn headers(hm: &http::HeaderMap) -> Result<Vec<(&str, &str)>> {
        let mut res = Vec::new();
        for (name, value) in hm
            .iter()
            .map(|(name, value)| (name.as_str(), std::str::from_utf8(value.as_bytes())))
        {
            let value = value?;
            res.push((name, value));
        }

        Ok(res)
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

    fn params(uri: &Uri) -> Result<Vec<(String, String)>> {
        match uri.query() {
            Some(q) => Ok(url::form_urlencoded::parse(q.as_bytes())
                .into_owned()
                .collect::<Vec<_>>()),
            None => Ok(vec![]),
        }
    }
}
