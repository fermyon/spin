//! Implementation for the Spin HTTP engine.

mod invoker;
mod routes;
mod spin;
mod wagi;

use anyhow::{Error, Result};
use async_trait::async_trait;
use http::StatusCode;
use hyper::{
    server::conn::AddrStream,
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use invoker::InternalInvoker;
use routes::Router;
use spin::SpinHttpExecutor;
use spin_config::{Configuration, CoreComponent, TriggerConfig};
use spin_engine::{Builder, ExecutionContextConfiguration};
use spin_http::SpinHttpData;
use std::{net::SocketAddr, sync::Arc};
use tracing::{instrument, log};
use wagi::WagiHttpExecutor;

wit_bindgen_wasmtime::import!("wit/ephemeral/spin-http.wit");

type ExecutionContext = spin_engine::ExecutionContext<HttpTriggerData>;
type RuntimeContext = spin_engine::RuntimeContext<HttpTriggerData>;

#[derive(Default)]
pub struct HttpTriggerData {
    pub invoker: InternalInvoker,
    pub http: SpinHttpData,
}

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

    invoker: InternalInvoker,
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

        let mut builder = Builder::new(config)?;
        builder.link_wasi()?;
        invoker::add_to_linker(
            &mut builder.linker,
            |ctx: &mut RuntimeContext| -> &mut InternalInvoker {
                &mut ctx.data.as_mut().unwrap().invoker
            },
        )?;

        let engine = Arc::new(builder.build()?);

        let invoker = InternalInvoker {
            app: app.clone(),
            engine: engine.clone(),
        };

        // let engine = Arc::new(Builder::build_default(config)?);
        let router = Router::build(&app)?;
        log::info!("Created new HTTP trigger.");

        Ok(Self {
            address,
            app,
            router,
            engine,
            invoker,
        })
    }

    /// Handle incoming requests using an HTTP executor.
    pub(crate) async fn handle(
        &self,
        req: Request<Body>,
        addr: SocketAddr,
    ) -> Result<Response<Body>> {
        log::info!(
            "Processing request for application {} on URI {}",
            &self.app.info.name,
            req.uri()
        );

        match req.uri().path() {
            "/healthz" => Ok(Response::new(Body::from("OK"))),
            route => match self.router.route(route) {
                Ok(c) => {
                    let TriggerConfig::Http(trigger) = &c.trigger;
                    let executor = match &trigger.executor {
                        Some(i) => i,
                        None => &spin_config::HttpExecutor::Spin,
                    };

                    let res = match executor {
                        spin_config::HttpExecutor::Spin => {
                            let invoker = &self.invoker;
                            let data = HttpTriggerData {
                                invoker: invoker.clone(),
                                http: SpinHttpData::default(),
                            };

                            SpinHttpExecutor::execute(&self.engine, Some(data), &c.id, req, addr)
                                .await
                        }
                        spin_config::HttpExecutor::Wagi => {
                            WagiHttpExecutor::execute(&self.engine, None, &c.id, req, addr).await
                        }
                    };
                    match res {
                        Ok(res) => return Ok(res),
                        Err(e) => {
                            log::error!("Error processing request: {:?}", e);
                            return Ok(Self::internal_error());
                        }
                    }
                }
                Err(_) => return Ok(Self::not_found()),
            },
        }
    }

    /// Create an HTTP 500 response.
    fn internal_error() -> Response<Body> {
        let mut err = Response::default();
        *err.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        err
    }

    /// Create an HTTP 404 response.
    fn not_found() -> Response<Body> {
        let mut not_found = Response::default();
        *not_found.status_mut() = StatusCode::NOT_FOUND;
        not_found
    }

    /// Run the HTTP trigger indefinitely.
    #[instrument(skip(self))]
    pub async fn run(&self) -> Result<()> {
        let mk_svc = make_service_fn(move |addr: &AddrStream| {
            let t = self.clone();
            let addr = addr.remote_addr();

            async move {
                Ok::<_, Error>(service_fn(move |req| {
                    let t2 = t.clone();

                    async move { t2.handle(req, addr).await }
                }))
            }
        });

        let addr: SocketAddr = self.address.parse()?;
        log::info!("Serving on address {:?}", addr);
        Server::bind(&addr).serve(mk_svc).await?;

        Ok(())
    }
}

/// The HTTP executor trait.
/// All HTTP executors must implement this trait.
#[async_trait]
pub(crate) trait HttpExecutor: Clone + Send + Sync + 'static {
    async fn execute(
        engine: &ExecutionContext,
        data: Option<HttpTriggerData>,
        component: &String,
        req: Request<Body>,
        client_addr: SocketAddr,
    ) -> Result<Response<Body>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use spin_config::{
        ApplicationInformation, Configuration, HttpConfig, HttpExecutor, ModuleSource,
        TriggerConfig,
    };
    use std::{
        net::{IpAddr, Ipv4Addr},
        sync::Once,
    };

    static LOGGER: Once = Once::new();

    const RUST_ENTRYPOINT_PATH: &str =
        "tests/rust-http-test/target/wasm32-wasi/release/rust_http_test.wasm";

    /// We can only initialize the tracing subscriber once per crate.
    pub(crate) fn init() {
        LOGGER.call_once(|| {
            tracing_subscriber::fmt()
                .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
                .init();
        });
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
                executor: Some(HttpExecutor::Spin),
            }),
            ..Default::default()
        };
        let components = vec![component];

        let cfg = Configuration::<CoreComponent> { info, components };
        let trigger = HttpTrigger::new("".to_string(), cfg, None)?;

        let body = Body::from("Fermyon".as_bytes().to_vec());
        let req = http::Request::builder()
            .method("POST")
            .uri("https://myservice.fermyon.dev/test?abc=def")
            .header("x-custom-foo", "bar")
            .header("x-custom-foo2", "bar2")
            .body(body)
            .unwrap();

        let res = trigger
            .handle(
                req,
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 1234),
            )
            .await?;
        assert_eq!(res.status(), StatusCode::OK);
        let body_bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
        assert_eq!(body_bytes.to_vec(), "Hello, Fermyon".as_bytes());

        Ok(())
    }
}
