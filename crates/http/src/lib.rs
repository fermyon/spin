//! Implementation for the Spin HTTP engine.

mod middleware;
mod routes;
mod spin;
mod wagi;

use crate::{middleware::MiddlewaresStack, wagi::WagiHttpExecutor};
use anyhow::{Error, Result};
use async_trait::async_trait;
use http::{StatusCode, Uri};
use hyper::{
    server::conn::AddrStream,
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use middleware::MiddlewareData;
use routes::{RoutePattern, Router};
use spin::SpinHttpExecutor;
use spin_config::{ApplicationTrigger, Configuration, CoreComponent, TriggerConfig};
use spin_engine::{Builder, ExecutionContextConfiguration};
use spin_http::SpinHttpData;
use std::{net::SocketAddr, sync::Arc};
use tracing::log;

wit_bindgen_wasmtime::import!({paths: ["wit/ephemeral/spin-http.wit"], async: *});

#[derive(Default)]
pub struct HttpData {
    http: SpinHttpData,
    middleware: MiddlewareData,
}

type ExecutionContext = spin_engine::ExecutionContext<HttpData>;
type RuntimeContext = spin_engine::RuntimeContext<HttpData>;

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
    pub async fn new(
        address: String,
        app: Configuration<CoreComponent>,
        wasmtime: Option<wasmtime::Config>,
    ) -> Result<Self> {
        let mut config = ExecutionContextConfiguration::new(app.clone());
        if let Some(wasmtime) = wasmtime {
            config.wasmtime = wasmtime;
        };

        let mut engine_builder = Builder::new(config)?;
        engine_builder.link_wasi()?;
        middleware::add_middleware_to_linker(&mut engine_builder.linker)?;
        let engine = Arc::new(engine_builder.build().await?);

        let router = Router::build(&app)?;
        log::debug!("Created new HTTP trigger.");

        Ok(Self {
            address,
            app,
            router,
            engine,
        })
    }

    /// Handle incoming requests using an HTTP executor.
    pub(crate) async fn handle(
        &self,
        mut req: Request<Body>,
        addr: SocketAddr,
    ) -> Result<Response<Body>> {
        log::info!(
            "Processing request for application {} on URI {}",
            &self.app.info.name,
            req.uri()
        );

        let ApplicationTrigger::Http(app_trigger) = &self.app.info.trigger.clone();

        match req.uri().path() {
            "/healthz" => Ok(Response::new(Body::from("OK"))),
            route => match self.router.route(route) {
                Ok(c) => {
                    let TriggerConfig::Http(trigger) = &c.trigger.unwrap();

                    let mut middleware_executor =
                        MiddlewaresStack::create(&self.engine, &c.middleware_ids)?;

                    req = match middleware_executor.execute_request_middlewares(req).await? {
                        middleware::RequestMiddlewareResult::Next(req) => req,
                        middleware::RequestMiddlewareResult::Stop(resp) => return Ok(resp),
                    };

                    let executor = match &trigger.executor {
                        Some(i) => i,
                        None => &spin_config::HttpExecutor::Spin,
                    };

                    let res = match executor {
                        spin_config::HttpExecutor::Spin => {
                            SpinHttpExecutor::execute(
                                &self.engine,
                                &c.id,
                                &app_trigger.base,
                                &trigger.route,
                                req,
                                addr,
                                &(),
                            )
                            .await
                        }
                        spin_config::HttpExecutor::Wagi(wagi_config) => {
                            WagiHttpExecutor::execute(
                                &self.engine,
                                &c.id,
                                &app_trigger.base,
                                &trigger.route,
                                req,
                                addr,
                                wagi_config,
                            )
                            .await
                        }
                    };

                    let res = match res {
                        Ok(resp) => middleware_executor.execute_response_middlewares(resp).await,
                        err => err,
                    };

                    res.or_else(|e| {
                        log::error!("Error processing request: {:?}", e);
                        Ok(Self::internal_error())
                    })
                }
                Err(_) => Ok(Self::not_found()),
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

        let shutdown_signal = on_ctrl_c()?;

        let addr: SocketAddr = self.address.parse()?;
        log::info!("Serving on address {:?}", addr);
        Server::bind(&addr)
            .serve(mk_svc)
            .with_graceful_shutdown(async {
                shutdown_signal.await.ok();
            })
            .await?;

        log::debug!("User requested shutdown: exiting");

        Ok(())
    }
}

fn on_ctrl_c() -> Result<impl std::future::Future<Output = Result<(), tokio::task::JoinError>>> {
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    ctrlc::set_handler(move || {
        tx.send(()).ok();
    })?;
    let rx_future = tokio::task::spawn_blocking(move || {
        rx.recv().ok();
    });
    Ok(rx_future)
}

// The default headers set across both executors.
const X_FULL_URL_HEADER: &str = "X_FULL_URL";
const PATH_INFO_HEADER: &str = "PATH_INFO";
const X_MATCHED_ROUTE_HEADER: &str = "X_MATCHED_ROUTE";
const X_COMPONENT_ROUTE_HEADER: &str = "X_COMPONENT_ROUTE";
const X_RAW_COMPONENT_ROUTE_HEADER: &str = "X_RAW_COMPONENT_ROUTE";
const X_BASE_PATH_HEADER: &str = "X_BASE_PATH";

pub(crate) fn default_headers(
    uri: &Uri,
    raw: &str,
    base: &str,
    host: &str,
    // scheme: &str,
) -> Result<Vec<(String, String)>> {
    let mut res = vec![];
    let abs_path = uri
        .path_and_query()
        .expect("cannot get path and query")
        .as_str();

    let path_info = RoutePattern::from(base, raw).relative(abs_path)?;

    // TODO: check if TLS is enabled and change the scheme to "https".
    let scheme = "http";
    let full_url = format!("{}://{}{}", scheme, host, abs_path);
    let matched_route = RoutePattern::sanitize_with_base(base, raw);

    res.push((PATH_INFO_HEADER.to_string(), path_info));
    res.push((X_FULL_URL_HEADER.to_string(), full_url));
    res.push((X_MATCHED_ROUTE_HEADER.to_string(), matched_route));

    res.push((X_BASE_PATH_HEADER.to_string(), base.to_string()));
    res.push((X_RAW_COMPONENT_ROUTE_HEADER.to_string(), raw.to_string()));
    res.push((
        X_COMPONENT_ROUTE_HEADER.to_string(),
        raw.to_string()
            .strip_suffix("/...")
            .unwrap_or(raw)
            .to_string(),
    ));

    Ok(res)
}

/// The HTTP executor trait.
/// All HTTP executors must implement this trait.
#[async_trait]
pub(crate) trait HttpExecutor: Clone + Send + Sync + 'static {
    /// Configuration specific to the implementor of this trait.
    type Config;

    async fn execute(
        engine: &ExecutionContext,
        component: &str,
        base: &str,
        raw_route: &str,
        req: Request<Body>,
        client_addr: SocketAddr,
        config: &Self::Config,
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
        collections::HashMap,
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

    fn fake_file_origin() -> spin_config::ApplicationOrigin {
        let dir = env!("CARGO_MANIFEST_DIR");
        let fake_path = std::path::PathBuf::from(dir).join("fake_spin.toml");
        spin_config::ApplicationOrigin::File(fake_path)
    }

    #[test]
    fn test_default_headers_with_base_path() -> Result<()> {
        let scheme = "https";
        let host = "fermyon.dev";
        let base = "/base";
        let trigger_route = "/foo/...";
        let component_path = "/foo";
        let path_info = "/bar";

        let req_uri = format!(
            "{}://{}{}{}{}?key1=value1&key2=value2",
            scheme, host, base, component_path, path_info
        );

        let req = http::Request::builder()
            .method("POST")
            .uri(req_uri)
            .body("")?;

        let default_headers = crate::default_headers(req.uri(), trigger_route, base, host)?;

        // TODO: we currently replace the scheme with HTTP. When TLS is supported, this should be fixed.
        assert_eq!(
            search(X_FULL_URL_HEADER, &default_headers).unwrap(),
            "http://fermyon.dev/base/foo/bar?key1=value1&key2=value2".to_string()
        );
        assert_eq!(
            search(PATH_INFO_HEADER, &default_headers).unwrap(),
            "/bar".to_string()
        );
        assert_eq!(
            search(X_MATCHED_ROUTE_HEADER, &default_headers).unwrap(),
            "/base/foo/...".to_string()
        );
        assert_eq!(
            search(X_BASE_PATH_HEADER, &default_headers).unwrap(),
            "/base".to_string()
        );
        assert_eq!(
            search(X_RAW_COMPONENT_ROUTE_HEADER, &default_headers).unwrap(),
            "/foo/...".to_string()
        );
        assert_eq!(
            search(X_COMPONENT_ROUTE_HEADER, &default_headers).unwrap(),
            "/foo".to_string()
        );

        Ok(())
    }

    #[test]
    fn test_default_headers_without_base_path() -> Result<()> {
        let scheme = "https";
        let host = "fermyon.dev";
        let base = "/";
        let trigger_route = "/foo/...";
        let component_path = "/foo";
        let path_info = "/bar";

        let req_uri = format!(
            "{}://{}{}{}?key1=value1&key2=value2",
            scheme, host, component_path, path_info
        );

        let req = http::Request::builder()
            .method("POST")
            .uri(req_uri)
            .body("")?;

        let default_headers = crate::default_headers(req.uri(), trigger_route, base, host)?;

        // TODO: we currently replace the scheme with HTTP. When TLS is supported, this should be fixed.
        assert_eq!(
            search(X_FULL_URL_HEADER, &default_headers).unwrap(),
            "http://fermyon.dev/foo/bar?key1=value1&key2=value2".to_string()
        );
        assert_eq!(
            search(PATH_INFO_HEADER, &default_headers).unwrap(),
            "/bar".to_string()
        );
        assert_eq!(
            search(X_MATCHED_ROUTE_HEADER, &default_headers).unwrap(),
            "/foo/...".to_string()
        );
        assert_eq!(
            search(X_BASE_PATH_HEADER, &default_headers).unwrap(),
            "/".to_string()
        );
        assert_eq!(
            search(X_RAW_COMPONENT_ROUTE_HEADER, &default_headers).unwrap(),
            "/foo/...".to_string()
        );
        assert_eq!(
            search(X_COMPONENT_ROUTE_HEADER, &default_headers).unwrap(),
            "/foo".to_string()
        );

        Ok(())
    }

    fn search(key: &str, headers: &[(String, String)]) -> Option<String> {
        let mut res: Option<String> = None;
        for (k, v) in headers {
            if k == key {
                res = Some(v.clone());
            }
        }

        res
    }

    #[tokio::test]
    async fn test_spin_http() -> Result<()> {
        init();

        let info = ApplicationInformation {
            api_version: "0.1.0".to_string(),
            name: "test-app".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            authors: vec![],
            trigger: spin_config::ApplicationTrigger::Http(spin_config::HttpTriggerConfiguration {
                base: "/".to_owned(),
            }),
            namespace: None,
            origin: fake_file_origin(),
        };

        let component = CoreComponent {
            source: ModuleSource::FileReference(RUST_ENTRYPOINT_PATH.into()),
            id: "test".to_string(),
            trigger: Some(TriggerConfig::Http(HttpConfig {
                route: "/test".to_string(),
                executor: Some(HttpExecutor::Spin),
            })),
            wasm: spin_config::WasmConfig {
                environment: HashMap::new(),
                mounts: vec![],
                allowed_http_hosts: vec![],
            },
            middleware_ids: vec![],
        };
        let components = vec![component];

        let cfg = Configuration::<CoreComponent> { info, components };
        let trigger = HttpTrigger::new("".to_string(), cfg, None).await?;

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
