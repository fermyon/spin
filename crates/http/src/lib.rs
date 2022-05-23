//! Implementation for the Spin HTTP engine.

mod routes;
mod spin;
mod tls;
mod wagi;

use spin_engine::io::FollowComponents;
use spin_manifest::{ComponentMap, HttpConfig, HttpTriggerConfiguration};
pub use tls::TlsConfig;

use crate::{
    routes::{RoutePattern, Router},
    spin::SpinHttpExecutor,
    wagi::WagiHttpExecutor,
};
use anyhow::{Context, Error, Result};
use async_trait::async_trait;
use futures_util::stream::StreamExt;
use http::{uri::Scheme, StatusCode, Uri};
use hyper::{
    server::accept,
    server::conn::AddrStream,
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use spin_http::SpinHttpData;
use spin_trigger::Trigger;
use std::{future::ready, net::SocketAddr, sync::Arc};
use tls_listener::TlsListener;
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::server::TlsStream;
use tracing::log;

wit_bindgen_wasmtime::import!("../../wit/ephemeral/spin-http.wit");

type ExecutionContext = spin_engine::ExecutionContext<SpinHttpData>;
type RuntimeContext = spin_engine::RuntimeContext<SpinHttpData>;

/// The Spin HTTP trigger.
///
/// Could this contain a list of multiple HTTP applications?
/// (there could be a field apps: HashMap<String, Config>, where
/// the key is the base path for the application, and the trigger
/// would work across multiple applications.)
#[derive(Clone)]
pub struct HttpTrigger {
    /// Trigger configuration.
    trigger_config: HttpTriggerConfiguration,
    /// Component trigger configurations.
    component_triggers: ComponentMap<HttpConfig>,
    /// Router.
    router: Router,
    /// Spin execution context.
    engine: Arc<ExecutionContext>,
    /// Which components should have their logs followed on stdout/stderr.
    follow: FollowComponents,
}

#[derive(Clone)]
pub struct HttpTriggerExecutionConfig {
    address: String,
    tls: Option<TlsConfig>,
}

impl HttpTriggerExecutionConfig {
    pub fn new(address: String, tls: Option<TlsConfig>) -> Self {
        Self { address, tls }
    }
}

#[async_trait]
impl Trigger for HttpTrigger {
    type ContextData = SpinHttpData;
    type Config = HttpTriggerConfiguration;
    type ComponentConfig = HttpConfig;
    type ExecutionConfig = HttpTriggerExecutionConfig;

    fn new(
        execution_context: ExecutionContext,
        trigger_config: Self::Config,
        component_triggers: ComponentMap<Self::ComponentConfig>,
        follow: FollowComponents,
    ) -> Result<Self> {
        let router = Router::build(&trigger_config.base, &component_triggers)?;
        log::trace!(
            "Constructed router for application {}: {:?}",
            execution_context.config.label,
            router.routes
        );

        Ok(Self {
            trigger_config,
            component_triggers,
            router,
            engine: Arc::new(execution_context),
            follow,
        })
    }

    /// Runs the HTTP trigger indefinitely.
    async fn run(&self, run_config: Self::ExecutionConfig) -> Result<()> {
        if let Some(ref tls) = run_config.tls {
            self.serve_tls(run_config.address, tls).await
        } else {
            self.serve(run_config.address).await
        }
    }
}

impl HttpTrigger {
    /// Handles incoming requests using an HTTP executor.
    pub async fn handle(&self, req: Request<Body>, addr: SocketAddr) -> Result<Response<Body>> {
        log::info!(
            "Processing request for application {} on URI {}",
            &self.engine.config.label,
            req.uri()
        );

        match req.uri().path() {
            "/healthz" => Ok(Response::new(Body::from("OK"))),
            route => match self.router.route(route) {
                Ok(component_id) => {
                    let trigger = self.component_triggers.get(component_id).unwrap();

                    let executor = match &trigger.executor {
                        Some(i) => i,
                        None => &spin_manifest::HttpExecutor::Spin,
                    };

                    let follow = self.follow.should_follow(component_id);

                    let res = match executor {
                        spin_manifest::HttpExecutor::Spin => {
                            let executor = SpinHttpExecutor;
                            executor
                                .execute(
                                    &self.engine,
                                    component_id,
                                    &self.trigger_config.base,
                                    &trigger.route,
                                    req,
                                    addr,
                                    follow,
                                )
                                .await
                        }
                        spin_manifest::HttpExecutor::Wagi(wagi_config) => {
                            let executor = WagiHttpExecutor {
                                wagi_config: wagi_config.clone(),
                            };
                            executor
                                .execute(
                                    &self.engine,
                                    component_id,
                                    &self.trigger_config.base,
                                    &trigger.route,
                                    req,
                                    addr,
                                    follow,
                                )
                                .await
                        }
                    };
                    match res {
                        Ok(res) => Ok(res),
                        Err(e) => {
                            log::error!("Error processing request: {:?}", e);
                            Self::internal_error(None)
                        }
                    }
                }
                Err(_) => Self::not_found(),
            },
        }
    }

    /// Creates an HTTP 500 response.
    fn internal_error(body: Option<&str>) -> Result<Response<Body>> {
        let body = match body {
            Some(body) => Body::from(body.as_bytes().to_vec()),
            None => Body::empty(),
        };

        Ok(Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(body)?)
    }

    /// Creates an HTTP 404 response.
    fn not_found() -> Result<Response<Body>> {
        let mut not_found = Response::default();
        *not_found.status_mut() = StatusCode::NOT_FOUND;
        Ok(not_found)
    }

    async fn serve(&self, address: String) -> Result<()> {
        let mk_svc = make_service_fn(move |addr: &AddrStream| {
            let t = self.clone();
            let addr = addr.remote_addr();

            async move {
                Ok::<_, Error>(service_fn(move |mut req| {
                    let t2 = t.clone();

                    async move {
                        match set_req_uri(&mut req, Scheme::HTTP) {
                            Ok(()) => t2.handle(req, addr).await,
                            Err(e) => {
                                log::warn!("{}", e);
                                Self::internal_error(Some("Socket connection error"))
                            }
                        }
                    }
                }))
            }
        });

        let addr: SocketAddr = address.parse()?;

        let server = Server::try_bind(&addr)
            .with_context(|| format!("Unable to listen on {}", addr))?
            .serve(mk_svc);

        println!("Serving HTTP on address http://{:?}", addr);
        log::info!("Serving HTTP on address {:?}", addr);

        println!("Available Routes:");
        for (route, component) in &self.router.routes {
            println!("  {}: http://{:?}{}", component, addr, route);
            if let Some(component) = self.engine.components.get(component) {
                if let Some(description) = &component.core.description {
                    println!("    {}", description);
                }
            }
        }

        // anyhow::bail!("A COMPLETELY FICTIONAL ERROR HAS OCCURRED DAVE");

        server.await?;
        // let shutdown_signal = on_ctrl_c()?;

        // tokio::select! {
        //     _ = server => {
        //         log::debug!("Server shut down: exiting");
        //     },
        //     _ = shutdown_signal => {
        //         log::debug!("User requested shutdown: exiting");
        //     },
        // };

        Ok(())
    }

    async fn serve_tls(&self, address: String, tls: &TlsConfig) -> Result<()> {
        let mk_svc = make_service_fn(move |conn: &TlsStream<TcpStream>| {
            let (inner, _) = conn.get_ref();
            let addr_res = inner.peer_addr().map_err(|e| e.to_string());
            let t = self.clone();

            Box::pin(async move {
                Ok::<_, Error>(service_fn(move |mut req| {
                    let t2 = t.clone();
                    let a_res = addr_res.clone();

                    async move {
                        match set_req_uri(&mut req, Scheme::HTTPS) {
                            Ok(()) => {}
                            Err(e) => {
                                log::warn!("{}", e);
                                return Self::internal_error(Some("Socket connection error"));
                            }
                        }

                        match a_res {
                            Ok(addr) => t2.handle(req, addr).await,
                            Err(e) => {
                                log::warn!("Socket connection error on new connection: {}", e);
                                Self::internal_error(Some("Socket connection error"))
                            }
                        }
                    }
                }))
            })
        });

        let addr: SocketAddr = address.parse()?;
        let listener = TcpListener::bind(&addr)
            .await
            .with_context(|| format!("Unable to listen on {}", addr))?;

        let tls_srv_cfg = tls.server_config()?;

        let incoming =
            accept::from_stream(TlsListener::new(tls_srv_cfg, listener).filter(|conn| {
                if let Err(err) = conn {
                    log::warn!("{:?}", err);
                    ready(false)
                } else {
                    ready(true)
                }
            }));

        let server = Server::builder(incoming).serve(mk_svc);

        println!("Serving HTTPS on address https://{:?}", addr);
        log::info!("Serving HTTPS on address {:?}", addr);

        server.await?;
        // let shutdown_signal = on_ctrl_c()?;

        // tokio::select! {
        //     _ = server => {
        //         log::debug!("Server shut down: exiting");
        //     },
        //     _ = shutdown_signal => {
        //         log::debug!("User requested shutdown: exiting");
        //     },
        // };

        Ok(())
    }
}

fn set_req_uri(req: &mut Request<Body>, scheme: Scheme) -> Result<()> {
    const DEFAULT_HOST: &str = "localhost";

    let authority_hdr = req
        .headers()
        .get(http::header::HOST)
        .map(|h| h.to_str().context("Expected UTF8 header value (authority)"))
        .unwrap_or(Ok(DEFAULT_HOST))?;
    let uri = req.uri().clone();
    let mut parts = uri.into_parts();
    parts.authority = authority_hdr
        .parse()
        .map(Option::Some)
        .map_err(|e| anyhow::anyhow!("Invalid authority {:?}", e))?;
    parts.scheme = Some(scheme);
    *req.uri_mut() = Uri::from_parts(parts).unwrap();
    Ok(())
}

// fn on_ctrl_c() -> Result<impl std::future::Future<Output = Result<(), tokio::task::JoinError>>> {
//     let (tx, rx) = std::sync::mpsc::channel::<()>();
//     ctrlc::set_handler(move || {
//         tx.send(()).ok();
//     })?;
//     let rx_future = tokio::task::spawn_blocking(move || {
//         rx.recv().ok();
//     });
//     Ok(rx_future)
// }

// We need to make the following pieces of information available to both executors.
// While the values we set are identical, the way they are passed to the
// modules is going to be different, so each executor must must use the info
// in its standardized way (environment variables for the Wagi executor, and custom headers
// for the Spin HTTP executor).
const FULL_URL: &[&str] = &["SPIN_FULL_URL", "X_FULL_URL"];
const PATH_INFO: &[&str] = &["SPIN_PATH_INFO", "PATH_INFO"];
const MATCHED_ROUTE: &[&str] = &["SPIN_MATCHED_ROUTE", "X_MATCHED_ROUTE"];
const COMPONENT_ROUTE: &[&str] = &["SPIN_COMPONENT_ROUTE", "X_COMPONENT_ROUTE"];
const RAW_COMPONENT_ROUTE: &[&str] = &["SPIN_RAW_COMPONENT_ROUTE", "X_RAW_COMPONENT_ROUTE"];
const BASE_PATH: &[&str] = &["SPIN_BASE_PATH", "X_BASE_PATH"];

pub(crate) fn compute_default_headers<'a>(
    uri: &Uri,
    raw: &str,
    base: &str,
    host: &str,
) -> Result<Vec<(&'a [&'a str], String)>> {
    let mut res = vec![];
    let abs_path = uri
        .path_and_query()
        .expect("cannot get path and query")
        .as_str();

    let path_info = RoutePattern::from(base, raw).relative(abs_path)?;

    let scheme = uri.scheme_str().unwrap_or("http");

    let full_url = format!("{}://{}{}", scheme, host, abs_path);
    let matched_route = RoutePattern::sanitize_with_base(base, raw);

    res.push((PATH_INFO, path_info));
    res.push((FULL_URL, full_url));
    res.push((MATCHED_ROUTE, matched_route));

    res.push((BASE_PATH, base.to_string()));
    res.push((RAW_COMPONENT_ROUTE, raw.to_string()));
    res.push((
        COMPONENT_ROUTE,
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
    // TODO: allowing this lint because I want to gather feedback before
    // investing time in reorganising this
    #[allow(clippy::too_many_arguments)]
    async fn execute(
        &self,
        engine: &ExecutionContext,
        component: &str,
        base: &str,
        raw_route: &str,
        req: Request<Body>,
        client_addr: SocketAddr,
        follow: bool,
    ) -> Result<Response<Body>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use spin_manifest::{HttpConfig, HttpExecutor};
    use spin_testing::test_socket_addr;
    use spin_trigger::build_trigger_from_app;
    use std::{collections::BTreeMap, sync::Once};

    static LOGGER: Once = Once::new();

    /// We can only initialize the tracing subscriber once per crate.
    pub(crate) fn init() {
        LOGGER.call_once(|| {
            tracing_subscriber::fmt()
                .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
                .with_ansi(atty::is(atty::Stream::Stderr))
                .init();
        });
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

        let default_headers = crate::compute_default_headers(req.uri(), trigger_route, base, host)?;

        assert_eq!(
            search(FULL_URL, &default_headers).unwrap(),
            "https://fermyon.dev/base/foo/bar?key1=value1&key2=value2".to_string()
        );
        assert_eq!(
            search(PATH_INFO, &default_headers).unwrap(),
            "/bar".to_string()
        );
        assert_eq!(
            search(MATCHED_ROUTE, &default_headers).unwrap(),
            "/base/foo/...".to_string()
        );
        assert_eq!(
            search(BASE_PATH, &default_headers).unwrap(),
            "/base".to_string()
        );
        assert_eq!(
            search(RAW_COMPONENT_ROUTE, &default_headers).unwrap(),
            "/foo/...".to_string()
        );
        assert_eq!(
            search(COMPONENT_ROUTE, &default_headers).unwrap(),
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

        let default_headers = crate::compute_default_headers(req.uri(), trigger_route, base, host)?;

        // TODO: we currently replace the scheme with HTTP. When TLS is supported, this should be fixed.
        assert_eq!(
            search(FULL_URL, &default_headers).unwrap(),
            "https://fermyon.dev/foo/bar?key1=value1&key2=value2".to_string()
        );
        assert_eq!(
            search(PATH_INFO, &default_headers).unwrap(),
            "/bar".to_string()
        );
        assert_eq!(
            search(MATCHED_ROUTE, &default_headers).unwrap(),
            "/foo/...".to_string()
        );
        assert_eq!(
            search(BASE_PATH, &default_headers).unwrap(),
            "/".to_string()
        );
        assert_eq!(
            search(RAW_COMPONENT_ROUTE, &default_headers).unwrap(),
            "/foo/...".to_string()
        );
        assert_eq!(
            search(COMPONENT_ROUTE, &default_headers).unwrap(),
            "/foo".to_string()
        );

        Ok(())
    }

    // fn _search(key: &str, headers: &[(String, String)]) -> Option<String> {}

    fn search<'a>(keys: &'a [&'a str], headers: &[(&[&str], String)]) -> Option<String> {
        let mut res: Option<String> = None;
        for (k, v) in headers {
            if k[0] == keys[0] && k[1] == keys[1] {
                res = Some(v.clone());
            }
        }

        res
    }

    #[tokio::test]
    async fn test_spin_http() -> Result<()> {
        init();

        let mut cfg = spin_testing::TestConfig::default();
        cfg.test_program("rust-http-test.wasm")
            .http_trigger(HttpConfig {
                route: "/test".to_string(),
                executor: Some(HttpExecutor::Spin),
            });
        let app = cfg.build_application();

        let trigger: HttpTrigger =
            build_trigger_from_app(app, None, FollowComponents::None, None).await?;

        let body = Body::from("Fermyon".as_bytes().to_vec());
        let req = http::Request::post("https://myservice.fermyon.dev/test?abc=def")
            .header("x-custom-foo", "bar")
            .header("x-custom-foo2", "bar2")
            .body(body)
            .unwrap();

        let res = trigger.handle(req, test_socket_addr()).await?;
        assert_eq!(res.status(), StatusCode::OK);
        let body_bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
        assert_eq!(body_bytes.to_vec(), "Hello, Fermyon".as_bytes());

        Ok(())
    }

    #[tokio::test]
    async fn test_wagi_http() -> Result<()> {
        init();

        let mut cfg = spin_testing::TestConfig::default();
        cfg.test_program("wagi-test.wasm").http_trigger(HttpConfig {
            route: "/test".to_string(),
            executor: Some(HttpExecutor::Wagi(Default::default())),
        });
        let app = cfg.build_application();

        let trigger: HttpTrigger =
            build_trigger_from_app(app, None, FollowComponents::None, None).await?;

        let body = Body::from("Fermyon".as_bytes().to_vec());
        let req = http::Request::builder()
            .method("POST")
            .uri("https://myservice.fermyon.dev/test?abc=def")
            .header("x-custom-foo", "bar")
            .header("x-custom-foo2", "bar2")
            .body(body)
            .unwrap();

        let res = trigger.handle(req, test_socket_addr()).await?;
        assert_eq!(res.status(), StatusCode::OK);
        let body_bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();

        #[derive(miniserde::Deserialize)]
        struct Env {
            args: Vec<String>,
            vars: BTreeMap<String, String>,
        }
        let env: Env =
            miniserde::json::from_str(std::str::from_utf8(body_bytes.as_ref()).unwrap()).unwrap();

        assert_eq!(env.args, ["/test", "abc=def"]);
        assert_eq!(env.vars["HTTP_X_CUSTOM_FOO"], "bar".to_string());

        Ok(())
    }
}
