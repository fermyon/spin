use crate::spin_http::{Method, SpinHttp};
use crate::HttpExecutor;
use crate::{ExecutionContext, RuntimeContext};
use anyhow::Result;
use async_trait::async_trait;
use hyper::{Body, Request, Response};
use std::str::FromStr;
use tracing::{instrument, log};
use url::Url;
use wasmtime::{Instance, Store};

#[derive(Clone)]
pub struct SpinHttpExecutor;

#[async_trait]
impl HttpExecutor for SpinHttpExecutor {
    #[instrument(skip(engine))]
    async fn execute(
        engine: &ExecutionContext,
        component: String,
        req: Request<Body>,
    ) -> Result<Response<Body>> {
        log::info!(
            "Executing request using the Spin executor for component {}",
            component
        );
        let (store, instance) = engine.prepare_component(component, None)?;
        let res = Self::execute_impl(store, instance, req).await?;
        log::info!("Request finished, sending response.");
        Ok(res)
    }
}

impl SpinHttpExecutor {
    pub async fn execute_impl(
        mut store: Store<RuntimeContext>,
        instance: Instance,
        req: Request<Body>,
    ) -> Result<Response<Body>> {
        let engine = SpinHttp::new(&mut store, &instance, |host| host.data.as_mut().unwrap())?;
        let (parts, bytes) = req.into_parts();
        let bytes = hyper::body::to_bytes(bytes).await?.to_vec();
        let body = Some(&bytes[..]);

        let method = Self::method(&parts.method);
        let uri = &parts.uri.to_string();
        let headers = &Self::headers(&parts.headers)?;
        // TODO
        // Currently, this silently crashes the running thread.
        // let params = &Self::params(&uri)?;
        // let params: &Vec<(&str, &str)> = &params.into_iter().map(|(k, v)| (&**k, &**v)).collect();
        let params = &Vec::new();
        let req = crate::spin_http::Request {
            method,
            uri,
            headers,
            params,
            body,
        };
        log::info!("Request URI: {:?}", req.uri);
        let res = engine.handler(&mut store, req)?;
        log::info!("Response status code: {:?}", res.status);
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

    #[allow(unused)]
    fn params(uri: &str) -> Result<Vec<(String, String)>> {
        let url = Url::parse(uri)?;
        Ok(url
            .query_pairs()
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect())
    }
}
