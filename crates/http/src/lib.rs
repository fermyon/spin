//! Implementation for the Spin HTTP engine.

pub mod routes;
mod spin;
mod tls;
mod wagi;

use std::{collections::HashMap, future::ready, net::SocketAddr, path::PathBuf, sync::Arc};

use anyhow::{Context, Error, Result};
use async_trait::async_trait;
use clap::Args;
use futures_util::stream::StreamExt;
use http::{uri::Scheme, StatusCode, Uri};
use hyper::{
    server::accept,
    server::conn::AddrStream,
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use serde::{Deserialize, Serialize};
use spin_trigger::{TriggerAppEngine, TriggerExecutor};
pub use tls::TlsConfig;
use tls_listener::TlsListener;
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::server::TlsStream;
use tracing::log;

pub use crate::wagi::WagiTriggerConfig;

use crate::{
    routes::{RoutePattern, Router},
    spin::SpinHttpExecutor,
    wagi::WagiHttpExecutor,
};

wit_bindgen_wasmtime::import!({paths: ["../../wit/ephemeral/spin-http.wit"], async: *});

pub(crate) type RuntimeData = spin_http::SpinHttpData;
pub(crate) type Store = spin_core::Store<RuntimeData>;

/// The Spin HTTP trigger.
pub struct HttpTrigger {
    app_engine: TriggerAppEngine<Self>,
    router: Router,
    component_trigger_configs: HashMap<String, HttpTriggerConfig>,
}

#[derive(Args)]
pub struct CliArgs {
    /// IP address and port to listen on
    #[clap(long = "listen", default_value = "127.0.0.1:3000")]
    pub address: String,

    /// The path to the certificate to use for https, if this is not set, normal http will be used. The cert should be in PEM format
    #[clap(long, env = "SPIN_TLS_CERT", requires = "tls-key")]
    pub tls_cert: Option<PathBuf>,

    /// The path to the certificate key to use for https, if this is not set, normal http will be used. The key should be in PKCS#8 format
    #[clap(long, env = "SPIN_TLS_KEY", requires = "tls-cert")]
    pub tls_key: Option<PathBuf>,
}

impl CliArgs {
    fn into_tls_config(self) -> Option<TlsConfig> {
        match (self.tls_cert, self.tls_key) {
            (Some(cert_path), Some(key_path)) => Some(TlsConfig {
                cert_path,
                key_path,
            }),
            (None, None) => None,
            _ => unreachable!(),
        }
    }
}

/// Configuration for the HTTP trigger
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HttpTriggerConfig {
    /// Component ID to invoke
    pub component: String,
    /// HTTP route the component will be invoked for
    pub route: String,
    /// The HTTP executor the component requires
    #[serde(default)]
    pub executor: Option<HttpExecutorType>,
}

/// The executor for the HTTP component.
/// The component can either implement the Spin HTTP interface,
/// or the Wagi CGI interface.
///
/// If an executor is not specified, the inferred default is `HttpExecutor::Spin`.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "lowercase", tag = "type")]
pub enum HttpExecutorType {
    /// The component implements the Spin HTTP interface.
    #[default]
    Spin,
    /// The component implements the Wagi CGI interface.
    Wagi(WagiTriggerConfig),
}

#[async_trait]
impl TriggerExecutor for HttpTrigger {
    const TRIGGER_TYPE: &'static str = "http";
    type RuntimeData = RuntimeData;
    type TriggerConfig = HttpTriggerConfig;
    type RunConfig = CliArgs;

    fn new(app_engine: TriggerAppEngine<Self>) -> Result<Self> {
        let component_routes = app_engine
            .trigger_configs()
            .map(|(_, config)| (config.component.clone(), config.route.clone()));
        let router = Router::build(component_routes)?;
        log::trace!(
            "Constructed router for application {}: {:?}",
            app_engine.app_name,
            router.routes
        );

        let component_trigger_configs = app_engine
            .trigger_configs()
            .map(|(_, config)| (config.component.clone(), config.clone()))
            .collect();

        Ok(Self {
            app_engine,
            router,
            component_trigger_configs,
        })
    }

    async fn run(self, config: Self::RunConfig) -> Result<()> {
        let listen_addr = config.address.parse()?;
        let tls = config.into_tls_config();

        // Print startup messages
        let scheme = if tls.is_some() { "https" } else { "http" };
        let base_url = format!("{}://{:?}", scheme, listen_addr);
        println!("Serving {}", base_url);
        log::info!("Serving {}", base_url);
        println!("Available Routes:");
        for (route, component_id) in &self.router.routes {
            println!("  {}: {}{}", component_id, base_url, route);
            if let Some(component) = self.app_engine.app().get_component(component_id) {
                if let Some(Ok(description)) = component.get_metadata::<&str>("description") {
                    println!("    {}", description);
                }
            }
        }

        if let Some(tls) = tls {
            self.serve_tls(listen_addr, tls).await?
        } else {
            self.serve(listen_addr).await?
        };
        Ok(())
    }
}

impl HttpTrigger {
    /// Handles incoming requests using an HTTP executor.
    pub async fn handle(
        &self,
        mut req: Request<Body>,
        scheme: Scheme,
        addr: SocketAddr,
    ) -> Result<Response<Body>> {
        set_req_uri(&mut req, scheme)?;

        log::info!(
            "Processing request for application {} on URI {}",
            &self.app_engine.app_name,
            req.uri()
        );

        match req.uri().path() {
            "/healthz" => Ok(Response::new(Body::from("OK"))),
            route => match self.router.route(route) {
                Ok(component_id) => {
                    let trigger = self.component_trigger_configs.get(component_id).unwrap();

                    let executor = trigger.executor.as_ref().unwrap_or(&HttpExecutorType::Spin);

                    let res = match executor {
                        HttpExecutorType::Spin => {
                            let executor = SpinHttpExecutor;
                            executor
                                .execute(&self.app_engine, component_id, &trigger.route, req, addr)
                                .await
                        }
                        HttpExecutorType::Wagi(wagi_config) => {
                            let executor = WagiHttpExecutor {
                                wagi_config: wagi_config.clone(),
                            };
                            executor
                                .execute(&self.app_engine, component_id, &trigger.route, req, addr)
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

    async fn serve(self, listen_addr: SocketAddr) -> Result<()> {
        let self_ = Arc::new(self);
        let make_service = make_service_fn(|conn: &AddrStream| {
            let self_ = self_.clone();
            let addr = conn.remote_addr();
            async move {
                let service = service_fn(move |req| {
                    let self_ = self_.clone();
                    async move { self_.handle(req, Scheme::HTTP, addr).await }
                });
                Ok::<_, Error>(service)
            }
        });

        Server::try_bind(&listen_addr)
            .with_context(|| format!("Unable to listen on {}", listen_addr))?
            .serve(make_service)
            .await?;
        Ok(())
    }

    async fn serve_tls(self, listen_addr: SocketAddr, tls: TlsConfig) -> Result<()> {
        let self_ = Arc::new(self);
        let make_service = make_service_fn(|conn: &TlsStream<TcpStream>| {
            let self_ = self_.clone();
            let (inner_conn, _) = conn.get_ref();
            let addr_res = inner_conn.peer_addr().map_err(|err| err.to_string());

            async move {
                let service = service_fn(move |req| {
                    let self_ = self_.clone();
                    let addr_res = addr_res.clone();

                    async move {
                        match addr_res {
                            Ok(addr) => self_.handle(req, Scheme::HTTPS, addr).await,
                            Err(err) => {
                                log::warn!("Failed to get remote socket address: {}", err);
                                Self::internal_error(Some("Socket connection error"))
                            }
                        }
                    }
                });
                Ok::<_, Error>(service)
            }
        });

        let listener = TcpListener::bind(&listen_addr)
            .await
            .with_context(|| format!("Unable to listen on {}", listen_addr))?;

        let incoming = accept::from_stream(
            TlsListener::new(tls.server_config()?, listener).filter(|conn| {
                if let Err(err) = conn {
                    log::warn!("{:?}", err);
                    ready(false)
                } else {
                    ready(true)
                }
            }),
        );

        Server::builder(incoming).serve(make_service).await?;
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

pub(crate) fn compute_default_headers(
    uri: &Uri,
    raw: &str,
    host: &str,
) -> Result<Vec<(&'static [&'static str], String)>> {
    let mut res = vec![];
    let abs_path = uri
        .path_and_query()
        .expect("cannot get path and query")
        .as_str();
    let path_info = RoutePattern::from(raw).relative(abs_path)?;

    let scheme = uri.scheme_str().unwrap_or("http");

    let full_url = format!("{}://{}{}", scheme, host, abs_path);
    let matched_route = RoutePattern::sanitize(raw);

    res.push((PATH_INFO, path_info));
    res.push((FULL_URL, full_url));
    res.push((MATCHED_ROUTE, matched_route));

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
    // investing time in reorganizing this
    #[allow(clippy::too_many_arguments)]
    async fn execute(
        &self,
        app_engine: &TriggerAppEngine<HttpTrigger>,
        component_id: &str,
        raw_route: &str,
        req: Request<Body>,
        client_addr: SocketAddr,
    ) -> Result<Response<Body>>;
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use anyhow::Result;
    use spin_testing::test_socket_addr;

    use super::*;

    #[test]
    fn test_default_headers_with_base_path() -> Result<()> {
        let scheme = "https";
        let host = "fermyon.dev";
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

        let default_headers = crate::compute_default_headers(req.uri(), trigger_route, host)?;

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

        let default_headers = crate::compute_default_headers(req.uri(), trigger_route, host)?;

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
            search(RAW_COMPONENT_ROUTE, &default_headers).unwrap(),
            "/foo/...".to_string()
        );
        assert_eq!(
            search(COMPONENT_ROUTE, &default_headers).unwrap(),
            "/foo".to_string()
        );

        Ok(())
    }

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
        let trigger = spin_testing::TestConfig::default()
            .test_program("rust-http-test.wasm")
            .http_spin_trigger("/test")
            .build_http_trigger()
            .await;

        let body = Body::from("Fermyon".as_bytes().to_vec());
        let req = http::Request::post("https://myservice.fermyon.dev/test?abc=def")
            .header("x-custom-foo", "bar")
            .header("x-custom-foo2", "bar2")
            .body(body)
            .unwrap();

        let res = trigger
            .handle(req, Scheme::HTTPS, test_socket_addr())
            .await?;
        assert_eq!(res.status(), StatusCode::OK);
        let body_bytes = hyper::body::to_bytes(res.into_body()).await.unwrap();
        assert_eq!(body_bytes.to_vec(), "Hello, Fermyon".as_bytes());

        Ok(())
    }

    #[tokio::test]
    async fn test_wagi_http() -> Result<()> {
        let trigger = spin_testing::TestConfig::default()
            .test_program("wagi-test.wasm")
            .http_wagi_trigger("/test", Default::default())
            .build_http_trigger()
            .await;

        let body = Body::from("Fermyon".as_bytes().to_vec());
        let req = http::Request::builder()
            .method("POST")
            .uri("https://myservice.fermyon.dev/test?abc=def")
            .header("x-custom-foo", "bar")
            .header("x-custom-foo2", "bar2")
            .body(body)
            .unwrap();

        let res = trigger
            .handle(req, Scheme::HTTPS, test_socket_addr())
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
