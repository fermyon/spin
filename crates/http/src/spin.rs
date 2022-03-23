use crate::{
    spin_http::{Method, SpinHttp},
    ExecutionContext, HttpExecutor, RuntimeContext,
};
use anyhow::Result;
use async_trait::async_trait;
use http::Uri;
use hyper::{Body, Request, Response};
use spin_engine::io::{IoStreamRedirects, OutRedirect};
use std::{
    net::SocketAddr,
    str,
    str::FromStr,
    sync::{Arc, RwLock},
};
use tokio::task::spawn_blocking;
use tracing::log;
use wasi_common::pipe::{ReadPipe, WritePipe};
use wasmtime::{Instance, Store};

#[derive(Clone)]
pub struct SpinHttpExecutor;

#[async_trait]
impl HttpExecutor for SpinHttpExecutor {
    async fn execute(
        &self,
        engine: &ExecutionContext,
        component: &str,
        base: &str,
        raw_route: &str,
        req: Request<Body>,
        _client_addr: SocketAddr,
    ) -> Result<Response<Body>> {
        log::trace!(
            "Executing request using the Spin executor for component {}",
            component
        );

        let io_redirects = prepare_io_redirects()?;

        let (store, instance) =
            engine.prepare_component(component, None, Some(io_redirects.clone()), None, None)?;

        let resp_result = Self::execute_impl(store, instance, base, raw_route, req).await;

        let log_result = engine.save_output_to_logs(io_redirects, component, true, true);

        // Defer checking for failures until here so that the logging runs
        // even if the guest code fails. (And when checking, check the guest
        // result first, so that guest failures are returned in preference to
        // log failures.)
        let resp = resp_result?;
        log_result?;

        log::info!(
            "Request finished, sending response with status code {}",
            resp.status()
        );
        Ok(resp)
    }
}

impl SpinHttpExecutor {
    pub async fn execute_impl(
        mut store: Store<RuntimeContext>,
        instance: Instance,
        base: &str,
        raw_route: &str,
        req: Request<Body>,
    ) -> Result<Response<Body>> {
        let headers;
        let mut req = req;
        {
            headers = Self::headers(&mut req, raw_route, base)?;
        }

        let engine = SpinHttp::new(&mut store, &instance, |host| host.data.as_mut().unwrap())?;
        let (parts, bytes) = req.into_parts();
        let bytes = hyper::body::to_bytes(bytes).await?.to_vec();

        let res = spawn_blocking(move || -> Result<crate::spin_http::Response> {
            let method = Self::method(&parts.method);

            let headers: Vec<(&str, &str)> = headers
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect();

            let params = &Self::params(&parts.uri)?;
            let params: Vec<(&str, &str)> = params
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect();

            let body = Some(&bytes[..]);

            let req = crate::spin_http::Request {
                method,
                uri: parts.uri.path(),
                headers: &headers,
                params: &params,
                body,
            };

            Ok(engine.handle_http_request(&mut store, req)?)
        })
        .await??;

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

    fn headers(req: &mut Request<Body>, raw: &str, base: &str) -> Result<Vec<(String, String)>> {
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

        // Add the default headers.
        for pair in crate::default_headers(req.uri(), raw, base, host)? {
            res.push(pair);
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

pub fn prepare_io_redirects() -> Result<IoStreamRedirects> {
    let stdin = ReadPipe::from(vec![]);

    let stdout_buf: Vec<u8> = vec![];
    let lock = Arc::new(RwLock::new(stdout_buf));
    let stdout = WritePipe::from_shared(lock.clone());
    let stdout = OutRedirect { out: stdout, lock };

    let stderr_buf: Vec<u8> = vec![];
    let lock = Arc::new(RwLock::new(stderr_buf));
    let stderr = WritePipe::from_shared(lock.clone());
    let stderr = OutRedirect { out: stderr, lock };

    Ok(IoStreamRedirects {
        stdin,
        stdout,
        stderr,
    })
}
