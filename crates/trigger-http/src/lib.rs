//! Implementation for the Spin HTTP engine.

mod handler;
mod instrument;
mod tls;
mod wagi;

use std::{
    collections::HashMap,
    io::IsTerminal,
    net::{Ipv4Addr, SocketAddr, ToSocketAddrs},
    path::PathBuf,
    str::FromStr,
    sync::Arc,
};

use anyhow::{Context, Result};
use async_trait::async_trait;
use clap::Args;
use http::{header::HOST, uri::Scheme, HeaderValue, StatusCode, Uri};
use http_body_util::BodyExt;
use hyper::{
    body::{Bytes, Incoming},
    server::conn::http1,
    service::service_fn,
    Request, Response,
};
use hyper_util::rt::tokio::TokioIo;
use instrument::{finalize_http_span, http_span};
use spin_app::{AppComponent, APP_DESCRIPTION_KEY};
use spin_core::{Engine, OutboundWasiHttpHandler};
use spin_http::{
    app_info::AppInfo,
    body,
    config::{HttpExecutorType, HttpTriggerConfig, HttpTriggerRouteConfig},
    routes::{RoutePattern, Router},
};
use spin_outbound_networking::{
    is_service_chaining_host, parse_service_chaining_target, AllowedHostsConfig, OutboundUrl,
};
use spin_trigger::{TriggerAppEngine, TriggerExecutor, TriggerInstancePre};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpListener,
    task,
};
use tracing::{field::Empty, log, Instrument};
use wasmtime_wasi_http::{
    body::{HyperIncomingBody as Body, HyperOutgoingBody},
    types::HostFutureIncomingResponse,
    WasiHttpView,
};

use crate::{
    handler::HttpHandlerExecutor,
    instrument::{instrument_error, MatchedRoute},
    wagi::WagiHttpExecutor,
};

pub use tls::TlsConfig;

pub(crate) type RuntimeData = HttpRuntimeData;
pub(crate) type Store = spin_core::Store<RuntimeData>;

/// The Spin HTTP trigger.
pub struct HttpTrigger {
    engine: Arc<TriggerAppEngine<Self>>,
    router: Router,
    // Base path for component routes.
    base: String,
    // Component ID -> component trigger config
    component_trigger_configs: HashMap<String, HttpTriggerConfig>,
}

#[derive(Args)]
pub struct CliArgs {
    /// IP address and port to listen on
    #[clap(long = "listen", default_value = "127.0.0.1:3000", value_parser = parse_listen_addr)]
    pub address: SocketAddr,

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

pub enum HttpInstancePre {
    Component(spin_core::InstancePre<RuntimeData>),
    Module(spin_core::ModuleInstancePre<RuntimeData>),
}

pub enum HttpInstance {
    Component(spin_core::Instance),
    Module(spin_core::ModuleInstance),
}

#[async_trait]
impl TriggerExecutor for HttpTrigger {
    const TRIGGER_TYPE: &'static str = "http";
    type RuntimeData = RuntimeData;
    type TriggerConfig = HttpTriggerConfig;
    type RunConfig = CliArgs;
    type InstancePre = HttpInstancePre;

    async fn new(engine: TriggerAppEngine<Self>) -> Result<Self> {
        let mut base = engine
            .trigger_metadata::<spin_http::trigger::Metadata>()?
            .unwrap_or_default()
            .base;

        if !base.starts_with('/') {
            base = format!("/{base}");
        }

        let component_routes = engine
            .trigger_configs()
            .map(|(_, config)| (config.component.as_str(), &config.route));

        let (router, duplicate_routes) = Router::build(&base, component_routes)?;

        if !duplicate_routes.is_empty() {
            log::error!("The following component routes are duplicates and will never be used:");
            for dup in &duplicate_routes {
                log::error!(
                    "  {}: {} (duplicate of {})",
                    dup.replaced_id,
                    dup.route.full_pattern_non_empty(),
                    dup.effective_id,
                );
            }
        }

        log::trace!(
            "Constructed router for application {}: {:?}",
            engine.app_name,
            router.routes().collect::<Vec<_>>()
        );

        let component_trigger_configs = engine
            .trigger_configs()
            .map(|(_, config)| (config.component.clone(), config.clone()))
            .collect();

        Ok(Self {
            engine: Arc::new(engine),
            router,
            base,
            component_trigger_configs,
        })
    }

    async fn run(self, config: Self::RunConfig) -> Result<()> {
        let listen_addr = config.address;
        let tls = config.into_tls_config();

        // Print startup messages
        let scheme = if tls.is_some() { "https" } else { "http" };
        let base_url = format!("{}://{:?}", scheme, listen_addr);
        terminal::step!("\nServing", "{}", base_url);
        log::info!("Serving {}", base_url);

        println!("Available Routes:");
        for (route, component_id) in self.router.routes() {
            println!("  {}: {}{}", component_id, base_url, route);
            if let Some(component) = self.engine.app().get_component(component_id) {
                if let Some(description) = component.get_metadata(APP_DESCRIPTION_KEY)? {
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

    fn supported_host_requirements() -> Vec<&'static str> {
        vec![spin_app::locked::SERVICE_CHAINING_KEY]
    }
}

#[async_trait]
impl TriggerInstancePre<RuntimeData, HttpTriggerConfig> for HttpInstancePre {
    type Instance = HttpInstance;

    async fn instantiate_pre(
        engine: &Engine<RuntimeData>,
        component: &AppComponent,
        config: &HttpTriggerConfig,
    ) -> Result<HttpInstancePre> {
        if let Some(HttpExecutorType::Wagi(_)) = &config.executor {
            let module = component.load_module(engine).await?;
            Ok(HttpInstancePre::Module(
                engine.module_instantiate_pre(&module)?,
            ))
        } else {
            let comp = component.load_component(engine).await?;
            Ok(HttpInstancePre::Component(engine.instantiate_pre(&comp)?))
        }
    }

    async fn instantiate(&self, store: &mut Store) -> Result<HttpInstance> {
        match self {
            HttpInstancePre::Component(pre) => pre
                .instantiate_async(store)
                .await
                .map(HttpInstance::Component),
            HttpInstancePre::Module(pre) => {
                pre.instantiate_async(store).await.map(HttpInstance::Module)
            }
        }
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
        strip_forbidden_headers(&mut req);

        spin_telemetry::extract_trace_context(&req);

        log::info!(
            "Processing request for application {} on URI {}",
            &self.engine.app_name,
            req.uri()
        );

        let path = req.uri().path().to_string();

        // Handle well-known spin paths
        if let Some(well_known) = path.strip_prefix(spin_http::WELL_KNOWN_PREFIX) {
            return match well_known {
                "health" => Ok(MatchedRoute::with_response_extension(
                    Response::new(body::full(Bytes::from_static(b"OK"))),
                    path,
                )),
                "info" => self.app_info(path),
                _ => Self::not_found(NotFoundRouteKind::WellKnown),
            };
        }

        // Route to app component
        match self.router.route(&path) {
            Ok(component_id) => {
                let trigger = self.component_trigger_configs.get(component_id).unwrap();

                let executor = trigger.executor.as_ref().unwrap_or(&HttpExecutorType::Http);

                let raw_route = match &trigger.route {
                    HttpTriggerRouteConfig::Route(r) => r.as_str(),
                    HttpTriggerRouteConfig::IsRoutable(_) => "/...",
                };

                let res = match executor {
                    HttpExecutorType::Http => {
                        HttpHandlerExecutor
                            .execute(
                                self.engine.clone(),
                                component_id,
                                &self.base,
                                raw_route,
                                req,
                                addr,
                            )
                            .await
                    }
                    HttpExecutorType::Wagi(wagi_config) => {
                        let executor = WagiHttpExecutor {
                            wagi_config: wagi_config.clone(),
                        };
                        executor
                            .execute(
                                self.engine.clone(),
                                component_id,
                                &self.base,
                                raw_route,
                                req,
                                addr,
                            )
                            .await
                    }
                };
                match res {
                    Ok(res) => Ok(MatchedRoute::with_response_extension(
                        res,
                        raw_route.to_string(),
                    )),
                    Err(e) => {
                        log::error!("Error processing request: {:?}", e);
                        instrument_error(&e);
                        Self::internal_error(None, raw_route.to_string())
                    }
                }
            }
            Err(_) => Self::not_found(NotFoundRouteKind::Normal(path.to_string())),
        }
    }

    /// Returns spin status information.
    fn app_info(&self, route: String) -> Result<Response<Body>> {
        let info = AppInfo::new(self.engine.app());
        let body = serde_json::to_vec_pretty(&info)?;
        Ok(MatchedRoute::with_response_extension(
            Response::builder()
                .header("content-type", "application/json")
                .body(body::full(body.into()))?,
            route,
        ))
    }

    /// Creates an HTTP 500 response.
    fn internal_error(body: Option<&str>, route: String) -> Result<Response<Body>> {
        let body = match body {
            Some(body) => body::full(Bytes::copy_from_slice(body.as_bytes())),
            None => body::empty(),
        };

        Ok(MatchedRoute::with_response_extension(
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(body)?,
            route,
        ))
    }

    /// Creates an HTTP 404 response.
    fn not_found(kind: NotFoundRouteKind) -> Result<Response<Body>> {
        use std::sync::atomic::{AtomicBool, Ordering};
        static SHOWN_GENERIC_404_WARNING: AtomicBool = AtomicBool::new(false);
        if let NotFoundRouteKind::Normal(route) = kind {
            if !SHOWN_GENERIC_404_WARNING.fetch_or(true, Ordering::Relaxed)
                && std::io::stderr().is_terminal()
            {
                terminal::warn!("Request to {route} matched no pattern, and received a generic 404 response. To serve a more informative 404 page, add a catch-all (/...) route.");
            }
        }
        Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(body::empty())?)
    }

    fn serve_connection<S: AsyncRead + AsyncWrite + Unpin + Send + 'static>(
        self_: Arc<Self>,
        stream: S,
        addr: SocketAddr,
    ) {
        task::spawn(async move {
            if let Err(e) = http1::Builder::new()
                .keep_alive(true)
                .serve_connection(
                    TokioIo::new(stream),
                    service_fn(move |request| self_.clone().instrumented_service_fn(addr, request)),
                )
                .await
            {
                log::warn!("{e:?}");
            }
        });
    }

    async fn instrumented_service_fn(
        self: Arc<Self>,
        addr: SocketAddr,
        request: Request<Incoming>,
    ) -> Result<Response<HyperOutgoingBody>> {
        let span = http_span!(request, addr);
        let method = request.method().to_string();
        async {
            let result = self
                .handle(
                    request.map(|body: Incoming| {
                        body.map_err(wasmtime_wasi_http::hyper_response_error)
                            .boxed()
                    }),
                    Scheme::HTTP,
                    addr,
                )
                .await;
            finalize_http_span(result, method)
        }
        .instrument(span)
        .await
    }

    async fn serve(self, listen_addr: SocketAddr) -> Result<()> {
        let self_ = Arc::new(self);

        let listener = TcpListener::bind(listen_addr)
            .await
            .with_context(|| format!("Unable to listen on {}", listen_addr))?;

        loop {
            let (stream, addr) = listener.accept().await?;
            Self::serve_connection(self_.clone(), stream, addr);
        }
    }

    async fn serve_tls(self, listen_addr: SocketAddr, tls: TlsConfig) -> Result<()> {
        let self_ = Arc::new(self);

        let listener = TcpListener::bind(listen_addr)
            .await
            .with_context(|| format!("Unable to listen on {}", listen_addr))?;

        let acceptor = tls.server_config()?;

        loop {
            let (stream, addr) = listener.accept().await?;
            match acceptor.accept(stream).await {
                Ok(stream) => Self::serve_connection(self_.clone(), stream, addr),
                Err(err) => tracing::error!(?err, "Failed to start TLS session"),
            }
        }
    }
}

fn parse_listen_addr(addr: &str) -> anyhow::Result<SocketAddr> {
    let addrs: Vec<SocketAddr> = addr.to_socket_addrs()?.collect();
    // Prefer 127.0.0.1 over e.g. [::1] because CHANGE IS HARD
    if let Some(addr) = addrs
        .iter()
        .find(|addr| addr.is_ipv4() && addr.ip() == Ipv4Addr::LOCALHOST)
    {
        return Ok(*addr);
    }
    // Otherwise, take the first addr (OS preference)
    addrs.into_iter().next().context("couldn't resolve address")
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

fn strip_forbidden_headers(req: &mut Request<Body>) {
    let headers = req.headers_mut();
    if let Some(host_header) = headers.get("Host") {
        if let Ok(host) = host_header.to_str() {
            if is_service_chaining_host(host) {
                headers.remove("Host");
            }
        }
    }
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
const BASE_PATH: &[&str] = &["SPIN_BASE_PATH", "X_BASE_PATH"];
const CLIENT_ADDR: &[&str] = &["SPIN_CLIENT_ADDR", "X_CLIENT_ADDR"];

pub(crate) fn compute_default_headers<'a>(
    uri: &Uri,
    raw: &str,
    base: &str,
    host: &str,
    client_addr: SocketAddr,
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
    res.push((CLIENT_ADDR, client_addr.to_string()));

    Ok(res)
}

/// The HTTP executor trait.
/// All HTTP executors must implement this trait.
#[async_trait]
pub(crate) trait HttpExecutor: Clone + Send + Sync + 'static {
    async fn execute(
        &self,
        engine: Arc<TriggerAppEngine<HttpTrigger>>,
        component_id: &str,
        base: &str,
        raw_route: &str,
        req: Request<Body>,
        client_addr: SocketAddr,
    ) -> Result<Response<Body>>;
}

#[derive(Clone)]
struct ChainedRequestHandler {
    engine: Arc<TriggerAppEngine<HttpTrigger>>,
    executor: HttpHandlerExecutor,
}

#[derive(Default)]
pub struct HttpRuntimeData {
    origin: Option<String>,
    chained_handler: Option<ChainedRequestHandler>,
    /// The hosts this app is allowed to make outbound requests to
    allowed_hosts: AllowedHostsConfig,
}

impl HttpRuntimeData {
    fn chain_request(
        data: &mut spin_core::Data<Self>,
        request: wasmtime_wasi_http::types::OutgoingRequest,
        component_id: String,
    ) -> wasmtime::Result<
        wasmtime::component::Resource<wasmtime_wasi_http::types::HostFutureIncomingResponse>,
    > {
        use wasmtime_wasi_http::types::IncomingResponseInternal;

        let this = data.as_ref();

        let chained_handler = this.chained_handler.clone().ok_or(wasmtime::Error::msg(
            "Internal error: internal request chaining not prepared (engine not assigned)",
        ))?;

        let engine = chained_handler.engine;
        let handler = chained_handler.executor;

        let base = "/";
        let raw_route = "/...";

        let client_addr = std::net::SocketAddr::from_str("0.0.0.0:0")?;

        let between_bytes_timeout = request.between_bytes_timeout;
        // The IncomingResponseInternal type formally needs a "worker" join handle, but
        // in a chained use case it doesn't seem to do anything.
        let worker = Arc::new(wasmtime_wasi::preview2::spawn(async {}));

        let req = request.request;

        let resp_fut = async move {
            match handler
                .execute(
                    engine.clone(),
                    &component_id,
                    base,
                    raw_route,
                    req,
                    client_addr,
                )
                .await
            {
                Ok(resp) => Ok(Ok(IncomingResponseInternal {
                    resp,
                    between_bytes_timeout,
                    worker,
                })),
                Err(e) => Err(wasmtime::Error::msg(e)),
            }
        };

        let handle = wasmtime_wasi::preview2::spawn(resp_fut);
        Ok(data.table().push(HostFutureIncomingResponse::new(handle))?)
    }
}

fn parse_chaining_target(request: &wasmtime_wasi_http::types::OutgoingRequest) -> Option<String> {
    parse_service_chaining_target(request.request.uri())
}

impl OutboundWasiHttpHandler for HttpRuntimeData {
    fn send_request(
        data: &mut spin_core::Data<Self>,
        mut request: wasmtime_wasi_http::types::OutgoingRequest,
    ) -> wasmtime::Result<
        wasmtime::component::Resource<wasmtime_wasi_http::types::HostFutureIncomingResponse>,
    >
    where
        Self: Sized,
    {
        let this = data.as_mut();

        let is_relative_url = request
            .request
            .uri()
            .authority()
            .map(|a| a.host().trim() == "")
            .unwrap_or_default();
        if is_relative_url {
            // Origin must be set in the incoming http handler
            let origin = this.origin.clone().unwrap();
            let path_and_query = request
                .request
                .uri()
                .path_and_query()
                .map(|p| p.as_str())
                .unwrap_or("/");
            let uri: Uri = format!("{origin}{path_and_query}")
                .parse()
                // origin together with the path and query must be a valid URI
                .unwrap();
            let host = format!("{}:{}", uri.host().unwrap(), uri.port().unwrap());
            let headers = request.request.headers_mut();
            headers.insert(HOST, HeaderValue::from_str(&host)?);

            request.use_tls = uri
                .scheme()
                .map(|s| s == &Scheme::HTTPS)
                .unwrap_or_default();
            // We know that `uri` has an authority because we set it above
            request.authority = uri.authority().unwrap().as_str().to_owned();
            *request.request.uri_mut() = uri;
        }

        let uri = request.request.uri();
        let uri_string = uri.to_string();
        let unallowed_relative =
            is_relative_url && !this.allowed_hosts.allows_relative_url(&["http", "https"]);
        let unallowed_absolute = !is_relative_url
            && !this
                .allowed_hosts
                .allows(&OutboundUrl::parse(uri_string, "https")?);
        if unallowed_relative || unallowed_absolute {
            tracing::log::error!("Destination not allowed: {}", request.request.uri());
            let host = if unallowed_absolute {
                // Safe to unwrap because absolute urls have a host by definition.
                let host = uri.authority().map(|a| a.host()).unwrap();
                let port = uri.authority().map(|a| a.port()).unwrap();
                let port = match port {
                    Some(port_str) => port_str.to_string(),
                    None => uri
                        .scheme()
                        .and_then(|s| (s == &Scheme::HTTP).then_some(80))
                        .unwrap_or(443)
                        .to_string(),
                };
                terminal::warn!(
                    "A component tried to make a HTTP request to non-allowed host '{host}'."
                );
                let scheme = uri.scheme().unwrap_or(&Scheme::HTTPS);
                format!("{scheme}://{host}:{port}")
            } else {
                terminal::warn!("A component tried to make a HTTP request to the same component but it does not have permission.");
                "self".into()
            };
            eprintln!("To allow requests, add 'allowed_outbound_hosts = [\"{}\"]' to the manifest component section.", host);
            anyhow::bail!("destination-not-allowed (error 1)")
        }

        if let Some(component_id) = parse_chaining_target(&request) {
            return Self::chain_request(data, request, component_id);
        }

        // TODO: This is a temporary workaround to make sure that outbound task is instrumented.
        // Once Wasmtime gives us the ability to do the spawn ourselves we can just call .instrument
        // and won't have to do this workaround.
        let response_handle = wasmtime_wasi_http::types::default_send_request(data, request)?;
        let response = data.table().get_mut(&response_handle)?;
        *response = match std::mem::replace(response, HostFutureIncomingResponse::Consumed) {
            HostFutureIncomingResponse::Pending(handle) => {
                HostFutureIncomingResponse::Pending(wasmtime_wasi::preview2::spawn(
                    async move {
                        let res: std::prelude::v1::Result<
                            std::prelude::v1::Result<
                                wasmtime_wasi_http::types::IncomingResponseInternal,
                                wasmtime_wasi_http::bindings::http::types::ErrorCode,
                            >,
                            anyhow::Error,
                        > = handle.await;
                        if let Ok(Ok(res)) = &res {
                            tracing::Span::current()
                                .record("http.response.status_code", res.resp.status().as_u16());
                        }
                        res
                    }
                    .in_current_span(),
                ))
            }
            other => other,
        };

        Ok(response_handle)
    }
}

#[derive(Debug, PartialEq)]
enum NotFoundRouteKind {
    Normal(String),
    WellKnown,
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::*;

    #[test]
    fn test_default_headers_with_base_path() -> Result<()> {
        let scheme = "https";
        let host = "fermyon.dev";
        let base = "/base";
        let trigger_route = "/foo/...";
        let component_path = "/foo";
        let path_info = "/bar";
        let client_addr: SocketAddr = "127.0.0.1:8777".parse().unwrap();

        let req_uri = format!(
            "{}://{}{}{}{}?key1=value1&key2=value2",
            scheme, host, base, component_path, path_info
        );

        let req = http::Request::builder()
            .method("POST")
            .uri(req_uri)
            .body("")?;

        let default_headers =
            crate::compute_default_headers(req.uri(), trigger_route, base, host, client_addr)?;

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
        assert_eq!(
            search(CLIENT_ADDR, &default_headers).unwrap(),
            "127.0.0.1:8777".to_string()
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
        let client_addr: SocketAddr = "127.0.0.1:8777".parse().unwrap();

        let req_uri = format!(
            "{}://{}{}{}?key1=value1&key2=value2",
            scheme, host, component_path, path_info
        );

        let req = http::Request::builder()
            .method("POST")
            .uri(req_uri)
            .body("")?;

        let default_headers =
            crate::compute_default_headers(req.uri(), trigger_route, base, host, client_addr)?;

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
        assert_eq!(
            search(CLIENT_ADDR, &default_headers).unwrap(),
            "127.0.0.1:8777".to_string()
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

    #[test]
    fn parse_listen_addr_prefers_ipv4() {
        let addr = parse_listen_addr("localhost:12345").unwrap();
        assert_eq!(addr.ip(), Ipv4Addr::LOCALHOST);
        assert_eq!(addr.port(), 12345);
    }

    #[test]
    fn forbidden_headers_are_removed() {
        let mut req = Request::get("http://test.spin.internal")
            .header("Host", "test.spin.internal")
            .header("accept", "text/plain")
            .body(Default::default())
            .unwrap();

        strip_forbidden_headers(&mut req);

        assert_eq!(1, req.headers().len());
        assert!(req.headers().get("Host").is_none());

        let mut req = Request::get("http://test.spin.internal")
            .header("Host", "test.spin.internal:1234")
            .header("accept", "text/plain")
            .body(Default::default())
            .unwrap();

        strip_forbidden_headers(&mut req);

        assert_eq!(1, req.headers().len());
        assert!(req.headers().get("Host").is_none());
    }

    #[test]
    fn non_forbidden_headers_are_not_removed() {
        let mut req = Request::get("http://test.example.com")
            .header("Host", "test.example.org")
            .header("accept", "text/plain")
            .body(Default::default())
            .unwrap();

        strip_forbidden_headers(&mut req);

        assert_eq!(2, req.headers().len());
        assert!(req.headers().get("Host").is_some());
    }
}
