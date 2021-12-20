use anyhow::Error;
use async_trait::async_trait;
use hyper::{
    server::conn::AddrStream,
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use spin_http_v01::{Method, SpinHttpV01, SpinHttpV01Data};
use std::{net::SocketAddr, str::FromStr};
use std::{sync::Arc, time::Instant};
use url::Url;
use wasmtime::{Instance, Store};

wit_bindgen_wasmtime::import!("crates/http/spin_http_v01.wai");

type ExecutionContext = spin_engine::ExecutionContext<SpinHttpV01Data>;
type RuntimeContext = spin_engine::RuntimeContext<SpinHttpV01Data>;

#[derive(Clone)]
pub struct HttpEngine(pub Arc<ExecutionContext>);

#[async_trait]
impl HttpService for HttpEngine {
    async fn execute(
        &self,
        req: hyper::Request<hyper::Body>,
    ) -> Result<hyper::Response<hyper::Body>, Error> {
        let start = Instant::now();
        let (store, instance) = self.0.prepare(None)?;
        let res = self.execute_impl(store, instance, req).await?;
        log::info!("Request execution time: {:#?}", start.elapsed());
        Ok(res)
    }
}

impl HttpEngine {
    pub async fn execute_impl(
        &self,
        mut store: Store<RuntimeContext>,
        instance: Instance,
        req: Request<Body>,
    ) -> Result<Response<Body>, Error> {
        let r = SpinHttpV01::new(&mut store, &instance, |host| host.data.as_mut().unwrap())?;

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

        let req = spin_http_v01::Request {
            method,
            uri,
            headers,
            params,
            body,
        };
        log::info!("Request URI: {:?}", req.uri);
        let res = r.handler(&mut store, req)?;
        log::info!("Response status code: {:?}", res.status);
        let mut fr = http::Response::builder().status(res.status);
        Self::append_headers(fr.headers_mut().unwrap(), res.headers)?;

        let body = match res.body {
            Some(b) => Body::from(b),
            None => Body::empty(),
        };

        Ok(fr.body(body)?)
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

    fn headers(hm: &http::HeaderMap) -> Result<Vec<(&str, &str)>, Error> {
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

    fn append_headers(
        res: &mut http::HeaderMap,
        src: Option<Vec<(String, String)>>,
    ) -> Result<(), Error> {
        match src {
            Some(src) => {
                for (k, v) in src.iter() {
                    res.insert(
                        http::header::HeaderName::from_str(k)?,
                        http::header::HeaderValue::from_str(v)?,
                    );
                }
            }
            None => {}
        };

        Ok(())
    }

    #[allow(unused)]
    fn params(uri: &str) -> Result<Vec<(String, String)>, Error> {
        let url = Url::parse(uri)?;
        Ok(url
            .query_pairs()
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect())
    }
}

#[async_trait]
pub trait HttpService: Clone + Send + Sync + 'static {
    async fn execute(&self, req: Request<Body>) -> Result<Response<Body>, Error>;
}

pub struct Trigger {
    pub address: String,
}

impl Trigger {
    pub async fn run(&self, runtime: impl HttpService) -> Result<(), Error> {
        let mk_svc = make_service_fn(move |_: &AddrStream| {
            let r = runtime.clone();
            async move {
                Ok::<_, Error>(service_fn(move |req| {
                    let r2 = r.clone();
                    async move { r2.execute(req).await }
                }))
            }
        });

        let addr: SocketAddr = self.address.parse()?;
        Server::bind(&addr).serve(mk_svc).await?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::HttpEngine;
    use crate::HttpService;
    use hyper::Body;
    use spin_engine::{Config, ExecutionContextBuilder};
    use std::sync::Arc;

    const RUST_ENTRYPOINT_PATH: &str =
        "tests/rust-http-test/target/wasm32-wasi/release/rust_http_test.wasm";

    #[tokio::test]
    async fn test_rust_hello() {
        let engine =
            ExecutionContextBuilder::build_default(RUST_ENTRYPOINT_PATH, Config::default())
                .unwrap();
        let engine = HttpEngine(Arc::new(engine));

        let body = Body::from("Fermyon".as_bytes().to_vec());
        let req = http::Request::builder()
            .method("POST")
            .uri("https://myservice.fermyon.dev")
            .header("X-Custom-Foo", "Bar")
            .header("X-Custom-Foo2", "Bar2")
            .body(body)
            .unwrap();

        let res = engine.execute(req).await.unwrap();
        let body_bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
        assert_eq!(body_bytes.to_vec(), "Hello, Fermyon".as_bytes());
    }
}
