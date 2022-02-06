//! Implementation for the Spin HTTP engine.

use anyhow::{Error, Result};
use async_trait::async_trait;
use http::StatusCode;
use hyper::{
    server::conn::AddrStream,
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use spin_config::{Configuration, CoreComponent};
use spin_engine::{Builder, ExecutionContextConfiguration};
use spin_http::{Method, SpinHttp, SpinHttpData};
use std::{collections::HashMap, net::SocketAddr, str::FromStr, sync::Arc};
use tracing::{instrument, log};
use url::Url;
use wasmtime::{Instance, Store};

wit_bindgen_wasmtime::import!("wit/ephemeral/spin-http.wit");

type ExecutionContext = spin_engine::ExecutionContext<SpinHttpData>;
type RuntimeContext = spin_engine::RuntimeContext<SpinHttpData>;

/// The Spin HTTP trigger.
/// TODO
/// This should contain TLS configuration.
///
/// Could this contain a list of multiple HTTP applications?
/// (there could be a field apps: HashMap<String, Config>, where
/// the key is the base path for the application, and the trigger
/// would work across multiple applications.)
#[derive(Clone)]
pub struct HttpTrigger {
    /// Listening address for the server.
    pub address: String,
    /// Configuration for the application.
    pub app: Configuration<CoreComponent>,
    /// Router.
    router: Router,
    /// Spin execution context.
    engine: Arc<ExecutionContext>,
}

impl HttpTrigger {
    /// Create a new Spin HTTP trigger.
    #[instrument]
    pub fn new(
        address: String,
        app: Configuration<CoreComponent>,
        wasmtime: Option<wasmtime::Config>,
    ) -> Result<Self> {
        let mut config = ExecutionContextConfiguration {
            app: app.clone(),
            ..Default::default()
        };
        if let Some(wasmtime) = wasmtime {
            config.wasmtime = wasmtime;
        };

        let engine = Arc::new(Builder::build_default(config)?);
        let router = Router::build(&app)?;
        log::info!("Created new HTTP trigger.");

        Ok(Self {
            address,
            app,
            router,
            engine,
        })
    }

    /// Handle an incoming request using an HTTP executor.
    pub async fn handle(&self, req: Request<Body>) -> Result<Response<Body>> {
        log::info!(
            "Processing requst for application {} on path {}",
            &self.app.info.name,
            req.uri().path()
        );

        match req.uri().path() {
            "/healthz" => Ok(Response::new(Body::from("OK"))),
            route => match self.router.routes.get(&route.to_string()) {
                Some(c) => return SpinHttpExecutor::execute(&self.engine, c.id.clone(), req).await,
                None => return Ok(Self::not_found()),
            },
        }
    }

    /// Create an HTTP 404 response
    fn not_found() -> Response<Body> {
        let mut not_found = Response::default();
        *not_found.status_mut() = StatusCode::NOT_FOUND;
        not_found
    }

    #[instrument(skip(self))]
    pub async fn run(&self) -> Result<()> {
        let mk_svc = make_service_fn(move |_: &AddrStream| {
            let t = self.clone();
            async move {
                Ok::<_, Error>(service_fn(move |req| {
                    let t2 = t.clone();
                    async move { t2.handle(req).await }
                }))
            }
        });

        let addr: SocketAddr = self.address.parse()?;
        log::info!("Serving on address {:?}", addr);
        Server::bind(&addr).serve(mk_svc).await?;

        Ok(())
    }
}

/// Router for the HTTP trigger.
#[derive(Clone)]
pub struct Router {
    /// Map between a path and the component that should handle it.
    pub routes: HashMap<String, CoreComponent>,
}

impl Router {
    /// Build a router based on application configuration.
    #[instrument]
    pub fn build(app: &Configuration<CoreComponent>) -> Result<Self> {
        let mut routes = HashMap::new();
        for component in &app.components {
            let spin_config::TriggerConfig::Http(trigger) = &component.trigger;
            log::info!("Trying route path {}", trigger.route);

            routes.insert(trigger.route.clone(), component.clone());
        }

        log::info!(
            "Constructed router for application {}: {:?}",
            app.info.name,
            routes
        );

        Ok(Self { routes })
    }
}

#[async_trait]
pub trait HttpExecutor: Clone + Send + Sync + 'static {
    async fn execute(
        engine: &ExecutionContext,
        component: String,
        req: Request<Body>,
    ) -> Result<Response<Body>>;
}

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
        log::info!("Executing request for component {}", component);
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
        let req = spin_http::Request {
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

// TODO
//
// Implement a Wagi executor.

#[derive(Clone)]
pub struct WagiHttpExecutor;

#[async_trait]
impl HttpExecutor for WagiHttpExecutor {
    #[instrument(skip(_engine))]
    async fn execute(
        _engine: &ExecutionContext,
        _component: String,
        _req: Request<Body>,
    ) -> Result<Response<Body>> {
        log::info!("Executing request for component {}", _component);
        todo!("Wagi executor not implemented yet.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use spin_config::{
        ApplicationInformation, Configuration, HttpConfig, HttpImplementation, ModuleSource,
        TriggerConfig,
    };

    const RUST_ENTRYPOINT_PATH: &str =
        "tests/rust-http-test/target/wasm32-wasi/release/rust_http_test.wasm";

    fn init() {
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .init();
    }

    #[tokio::test]
    #[instrument]
    async fn test_spin_http() -> Result<()> {
        init();

        let info = ApplicationInformation {
            name: "test-app".to_string(),
            version: "1.0.0".to_string(),
            ..Default::default()
        };

        let component = CoreComponent {
            source: ModuleSource::FileReference(RUST_ENTRYPOINT_PATH.into()),
            id: "test".to_string(),
            trigger: TriggerConfig::Http(HttpConfig {
                route: "/test".to_string(),
                implementation: Some(HttpImplementation::Spin),
            }),
            ..Default::default()
        };
        let components = vec![component];

        let cfg = Configuration::<CoreComponent> { info, components };
        let trigger = HttpTrigger::new("".to_string(), cfg, None)?;

        let body = Body::from("Fermyon".as_bytes().to_vec());
        let req = http::Request::builder()
            .method("POST")
            .uri("https://myservice.fermyon.dev/test")
            .header("X-Custom-Foo", "Bar")
            .header("X-Custom-Foo2", "Bar2")
            .body(body)
            .unwrap();

        let res = trigger.handle(req).await?;
        assert_eq!(res.status(), StatusCode::OK);
        let body_bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
        assert_eq!(body_bytes.to_vec(), "Hello, Fermyon".as_bytes());

        Ok(())
    }
}
