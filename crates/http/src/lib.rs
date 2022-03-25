//! Implementation for the Spin HTTP engine.

mod routes;
mod spin;
mod tls;
mod wagi;
pub use tls::TlsConfig;

use crate::{
    routes::{RoutePattern, Router},
    spin::SpinHttpExecutor,
    wagi::WagiHttpExecutor,
};
use anyhow::{anyhow, ensure, Context, Error, Result};
use async_trait::async_trait;
use futures_util::stream::StreamExt;
use http::{uri::Scheme, StatusCode, Uri};
use hyper::{
    server::accept,
    server::conn::AddrStream,
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use spin_config::{Configuration, CoreComponent};
use spin_engine::{Builder, ExecutionContextConfiguration};
use spin_http::SpinHttpData;
use std::{future::ready, net::SocketAddr, path::PathBuf, sync::Arc};
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
    /// Listening address for the server.
    address: String,
    /// Configuration for the application.
    app: Configuration<CoreComponent>,
    /// TLS configuration for the server.
    tls: Option<TlsConfig>,
    /// Router.
    router: Router,
    /// Spin execution context.
    engine: Arc<ExecutionContext>,
}

impl HttpTrigger {
    /// Creates a new Spin HTTP trigger.
    pub async fn new(
        address: String,
        app: Configuration<CoreComponent>,
        wasmtime: Option<wasmtime::Config>,
        tls: Option<TlsConfig>,
        log_dir: Option<PathBuf>,
    ) -> Result<Self> {
        ensure!(
            app.info.trigger.as_http().is_some(),
            "Application trigger is not HTTP"
        );

        let mut config = ExecutionContextConfiguration::new(app.clone(), log_dir);
        if let Some(wasmtime) = wasmtime {
            config.wasmtime = wasmtime;
        };

        let engine = Arc::new(Builder::build_default(config).await?);
        let router = Router::build(&app)?;
        log::trace!("Created new HTTP trigger.");

        Ok(Self {
            address,
            app,
            tls,
            router,
            engine,
        })
    }

    /// Handles incoming requests using an HTTP executor.
    pub async fn handle(&self, req: Request<Body>, addr: SocketAddr) -> Result<Response<Body>> {
        log::info!(
            "Processing request for application {} on URI {}",
            &self.app.info.name,
            req.uri()
        );

        // We can unwrap here because the trigger type has already been asserted in `HttpTrigger::new`
        let app_trigger = self.app.info.trigger.as_http().cloned().unwrap();

        match req.uri().path() {
            "/healthz" => Ok(Response::new(Body::from("OK"))),
            route => match self.router.route(route) {
                Ok(c) => {
                    let trigger = c.trigger.as_http().ok_or_else(|| {
                        anyhow!("Expected HTTP configuration for component {}", c.id)
                    })?;
                    let executor = match &trigger.executor {
                        Some(i) => i,
                        None => &spin_config::HttpExecutor::Spin,
                    };

                    let res = match executor {
                        spin_config::HttpExecutor::Spin => {
                            let executor = SpinHttpExecutor;
                            executor
                                .execute(
                                    &self.engine,
                                    &c.id,
                                    &app_trigger.base,
                                    &trigger.route,
                                    req,
                                    addr,
                                )
                                .await
                        }
                        spin_config::HttpExecutor::Wagi(wagi_config) => {
                            let executor = WagiHttpExecutor {
                                wagi_config: wagi_config.clone(),
                            };
                            executor
                                .execute(
                                    &self.engine,
                                    &c.id,
                                    &app_trigger.base,
                                    &trigger.route,
                                    req,
                                    addr,
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

    /// Runs the HTTP trigger indefinitely.
    pub async fn run(&self) -> Result<()> {
        match self.tls.as_ref() {
            Some(tls) => self.serve_tls(tls).await?,
            None => self.serve().await?,
        }
        Ok(())
    }

    async fn serve(&self) -> Result<()> {
        let mk_svc = make_service_fn(move |addr: &AddrStream| {
            let t = self.clone();
            let addr = addr.remote_addr();

            async move {
                Ok::<_, Error>(service_fn(move |mut req| {
                    let t2 = t.clone();

                    async move {
                        match set_req_uri(&mut req, Scheme::HTTPS) {
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

        let addr: SocketAddr = self.address.parse()?;

        let server = Server::try_bind(&addr)
            .with_context(|| format!("Unable to listen on {}", addr))?
            .serve(mk_svc);

        println!("Serving HTTP on address http://{:?}", addr);
        log::info!("Serving HTTP on address {:?}", addr);

        let shutdown_signal = on_ctrl_c()?;

        tokio::select! {
            _ = server => {
                log::debug!("Server shut down: exiting");
            },
            _ = shutdown_signal => {
                log::debug!("User requested shutdown: exiting");
            },
        };

        Ok(())
    }

    async fn serve_tls(&self, tls: &TlsConfig) -> Result<()> {
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

        let addr: SocketAddr = self.address.parse()?;
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

        let shutdown_signal = on_ctrl_c()?;

        tokio::select! {
            _ = server => {
                log::debug!("Server shut down: exiting");
            },
            _ = shutdown_signal => {
                log::debug!("User requested shutdown: exiting");
            },
        };

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
) -> Result<Vec<(String, String)>> {
    let mut res = vec![];
    let abs_path = uri
        .path_and_query()
        .expect("cannot get path and query")
        .as_str();

    let path_info = RoutePattern::from(base, raw).relative(abs_path)?;

    let scheme = uri.scheme_str().unwrap_or("http");

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
    async fn execute(
        &self,
        engine: &ExecutionContext,
        component: &str,
        base: &str,
        raw_route: &str,
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
        collections::BTreeMap,
        net::{IpAddr, Ipv4Addr},
        sync::Once,
    };

    static LOGGER: Once = Once::new();

    const RUST_ENTRYPOINT_PATH: &str = "../../target/test-programs/rust-http-test.wasm";

    const WAGI_ENTRYPOINT_PATH: &str = "../../target/test-programs/wagi-test.wasm";

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

        assert_eq!(
            search(X_FULL_URL_HEADER, &default_headers).unwrap(),
            "https://fermyon.dev/base/foo/bar?key1=value1&key2=value2".to_string()
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
            "https://fermyon.dev/foo/bar?key1=value1&key2=value2".to_string()
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
            spin_version: spin_config::SpinVersion::V1,
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
            trigger: TriggerConfig::Http(HttpConfig {
                route: "/test".to_string(),
                executor: Some(HttpExecutor::Spin),
            }),
            wasm: Default::default(),
        };
        let components = vec![component];

        let cfg = Configuration::<CoreComponent> { info, components };
        let trigger = HttpTrigger::new("".to_string(), cfg, None, None, None).await?;

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

    #[tokio::test]
    async fn test_wagi_http() -> Result<()> {
        init();

        let info = ApplicationInformation {
            spin_version: spin_config::SpinVersion::V1,
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
            source: ModuleSource::FileReference(WAGI_ENTRYPOINT_PATH.into()),
            id: "test".to_string(),
            trigger: TriggerConfig::Http(HttpConfig {
                route: "/test".to_string(),
                executor: Some(HttpExecutor::Wagi(Default::default())),
            }),
            wasm: spin_config::WasmConfig::default(),
        };
        let components = vec![component];

        let cfg = Configuration::<CoreComponent> { info, components };
        let trigger = HttpTrigger::new("".to_string(), cfg, None, None, None).await?;

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
