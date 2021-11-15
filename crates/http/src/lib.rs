use anyhow::Error;
use async_trait::async_trait;
use fermyon_http_v01::{FermyonHttpV01, FermyonHttpV01Data, Method};
use hyper::{
    server::conn::AddrStream,
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use std::net::SocketAddr;
use std::{str::FromStr, sync::Arc, time::Instant};
use wasmtime::{Instance, Store};

wai_bindgen_wasmtime::import!("crates/http/fermyon_http_v01.wai");

type ExecutionContext = fermyon_engine::ExecutionContext<FermyonHttpV01Data>;
type RuntimeContext = fermyon_engine::RuntimeContext<FermyonHttpV01Data>;

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
        log::info!("Total request execution time: {:#?}", start.elapsed());
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
        let r = FermyonHttpV01::new(&mut store, &instance, |host| host.data.as_mut().unwrap())?;

        let m = match *req.method() {
            http::Method::GET => Method::Get,
            http::Method::POST => Method::Post,
            http::Method::PUT => Method::Put,
            http::Method::DELETE => Method::Delete,
            http::Method::PATCH => Method::Patch,
            _ => todo!(),
        };
        let u = req.uri().to_string();

        let headers = Self::header_map_to_vec(req.headers())?;
        let headers: Vec<&str> = headers.iter().map(|s| &**s).collect();

        let (_, b) = req.into_parts();
        let b = hyper::body::to_bytes(b).await?.to_vec();
        let req = (m, u.as_str(), &headers[..], None, Some(&b[..]));

        let (status, headers, body) = r.handler(&mut store, req)?;
        log::info!("Result status code: {}", status);
        let mut hr = http::Response::builder().status(status);
        Self::append_headers(hr.headers_mut().unwrap(), headers)?;

        let body = match body {
            Some(b) => Body::from(b),
            None => Body::empty(),
        };

        Ok(hr.body(body)?)
    }

    /// Generate a string vector from an HTTP header map.
    fn header_map_to_vec(hm: &http::HeaderMap) -> Result<Vec<String>, Error> {
        let mut res = Vec::new();
        for (name, value) in hm
            .iter()
            .map(|(name, value)| (name.as_str(), std::str::from_utf8(value.as_bytes())))
        {
            let value = value?;
            anyhow::ensure!(
                !name
                    .chars()
                    .any(|x| x.is_control() || "(),/:;<=>?@[\\]{}".contains(x)),
                "Invalid header name"
            );
            anyhow::ensure!(
                !value.chars().any(|x| x.is_control()),
                "Invalid header value"
            );
            res.push(format!("{}:{}", name, value));
        }
        Ok(res)
    }

    /// Append a header map string to a mutable http::HeaderMap.
    fn append_headers(
        res_headers: &mut http::HeaderMap,
        source: Option<Vec<String>>,
    ) -> Result<(), Error> {
        match source {
            Some(h) => {
                for pair in h {
                    let mut parts = pair.splitn(2, ':');
                    let k = parts.next().ok_or_else(|| {
                        anyhow::format_err!("Invalid serialized header: [{}]", pair)
                    })?;
                    let v = parts.next().unwrap();
                    res_headers.insert(
                        http::header::HeaderName::from_str(k)?,
                        http::header::HeaderValue::from_str(v)?,
                    );
                }

                Ok(())
            }
            None => Ok(()),
        }
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
