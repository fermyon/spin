//! Implementation for the Spin HTTP engine.

mod handler;
mod instrument;
mod tls;
mod wagi;

use std::{
    collections::HashMap,
    error::Error,
    io::IsTerminal,
    net::{Ipv4Addr, SocketAddr, ToSocketAddrs},
    path::PathBuf,
    str::FromStr,
    sync::Arc,
};

use anyhow::{Context, Result};
use async_trait::async_trait;
use clap::Args;
use http::{header::HOST, uri::Authority, uri::Scheme, HeaderValue, StatusCode, Uri};
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
    config::{HttpExecutorType, HttpTriggerConfig},
    routes::{RouteMatch, Router},
};
use spin_outbound_networking::{
    is_service_chaining_host, parse_service_chaining_target, AllowedHostsConfig, OutboundUrl,
};
use spin_trigger::{ParsedClientTlsOpts, TriggerAppEngine, TriggerExecutor, TriggerInstancePre};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::{TcpListener, TcpStream},
    task,
    time::timeout,
};

use tracing::{field::Empty, log, Instrument};
use wasmtime_wasi_http::{
    bindings::wasi::http::{types, types::ErrorCode},
    body::{HyperIncomingBody as Body, HyperOutgoingBody},
    types::HostFutureIncomingResponse,
    HttpError, HttpResult,
};

use crate::{
    handler::{HandlerType, HttpHandlerExecutor},
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
    #[clap(long = "listen", env = "SPIN_HTTP_LISTEN_ADDR", default_value = "127.0.0.1:3000", value_parser = parse_listen_addr)]
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
    Component(spin_core::InstancePre<RuntimeData>, HandlerType),
    Module(spin_core::ModuleInstancePre<RuntimeData>),
}

pub enum HttpInstance {
    Component(spin_core::Instance, HandlerType),
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
                    dup.route(),
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

        let listener = TcpListener::bind(listen_addr)
            .await
            .with_context(|| format!("Unable to listen on {}", listen_addr))?;

        let self_ = Arc::new(self);
        if let Some(tls) = tls {
            self_.serve_tls(listener, listen_addr, tls).await?
        } else {
            self_.serve(listener, listen_addr).await?
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
            let handler_ty = HandlerType::from_component(engine, &comp)?;
            Ok(HttpInstancePre::Component(
                engine.instantiate_pre(&comp)?,
                handler_ty,
            ))
        }
    }

    async fn instantiate(&self, store: &mut Store) -> Result<HttpInstance> {
        match self {
            HttpInstancePre::Component(pre, ty) => Ok(HttpInstance::Component(
                pre.instantiate_async(store).await?,
                *ty,
            )),
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
        server_addr: SocketAddr,
        client_addr: SocketAddr,
    ) -> Result<Response<Body>> {
        set_req_uri(&mut req, scheme, server_addr)?;
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
            Ok(route_match) => {
                spin_telemetry::metrics::monotonic_counter!(
                    spin.request_count = 1,
                    trigger_type = "http",
                    app_id = &self.engine.app_name,
                    component_id = route_match.component_id()
                );

                let component_id = route_match.component_id();

                let trigger = self.component_trigger_configs.get(component_id).unwrap();

                let executor = trigger.executor.as_ref().unwrap_or(&HttpExecutorType::Http);

                let res = match executor {
                    HttpExecutorType::Http => {
                        HttpHandlerExecutor
                            .execute(
                                self.engine.clone(),
                                &self.base,
                                &route_match,
                                req,
                                client_addr,
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
                                &self.base,
                                &route_match,
                                req,
                                client_addr,
                            )
                            .await
                    }
                };
                match res {
                    Ok(res) => Ok(MatchedRoute::with_response_extension(
                        res,
                        route_match.raw_route(),
                    )),
                    Err(e) => {
                        log::error!("Error processing request: {:?}", e);
                        instrument_error(&e);
                        Self::internal_error(None, route_match.raw_route())
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
    fn internal_error(body: Option<&str>, route: impl Into<String>) -> Result<Response<Body>> {
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
        self: Arc<Self>,
        stream: S,
        server_addr: SocketAddr,
        client_addr: SocketAddr,
    ) {
        task::spawn(async move {
            if let Err(e) = http1::Builder::new()
                .keep_alive(true)
                .serve_connection(
                    TokioIo::new(stream),
                    service_fn(move |request| {
                        self.clone()
                            .instrumented_service_fn(server_addr, client_addr, request)
                    }),
                )
                .await
            {
                log::warn!("{e:?}");
            }
        });
    }

    async fn instrumented_service_fn(
        self: Arc<Self>,
        server_addr: SocketAddr,
        client_addr: SocketAddr,
        request: Request<Incoming>,
    ) -> Result<Response<HyperOutgoingBody>> {
        let span = http_span!(request, client_addr);
        let method = request.method().to_string();
        async {
            let result = self
                .handle(
                    request.map(|body: Incoming| {
                        body.map_err(wasmtime_wasi_http::hyper_response_error)
                            .boxed()
                    }),
                    Scheme::HTTP,
                    server_addr,
                    client_addr,
                )
                .await;
            finalize_http_span(result, method)
        }
        .instrument(span)
        .await
    }

    async fn serve(self: Arc<Self>, listener: TcpListener, listen_addr: SocketAddr) -> Result<()> {
        self.print_startup_msgs("http", &listener)?;
        loop {
            let (stream, client_addr) = listener.accept().await?;
            self.clone()
                .serve_connection(stream, listen_addr, client_addr);
        }
    }

    async fn serve_tls(
        self: Arc<Self>,
        listener: TcpListener,
        listen_addr: SocketAddr,
        tls: TlsConfig,
    ) -> Result<()> {
        let acceptor = tls.server_config()?;
        self.print_startup_msgs("https", &listener)?;

        loop {
            let (stream, addr) = listener.accept().await?;
            match acceptor.accept(stream).await {
                Ok(stream) => self.clone().serve_connection(stream, listen_addr, addr),
                Err(err) => tracing::error!(?err, "Failed to start TLS session"),
            }
        }
    }

    fn print_startup_msgs(&self, scheme: &str, listener: &TcpListener) -> Result<()> {
        let local_addr = listener.local_addr()?;
        let base_url = format!("{scheme}://{local_addr:?}");
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
        Ok(())
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

/// The incoming request's scheme and authority
///
/// The incoming request's URI is relative to the server, so we need to set the scheme and authority
fn set_req_uri(req: &mut Request<Body>, scheme: Scheme, addr: SocketAddr) -> Result<()> {
    let uri = req.uri().clone();
    let mut parts = uri.into_parts();
    let authority = format!("{}:{}", addr.ip(), addr.port()).parse().unwrap();
    parts.scheme = Some(scheme);
    parts.authority = Some(authority);
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
const FULL_URL: [&str; 2] = ["SPIN_FULL_URL", "X_FULL_URL"];
const PATH_INFO: [&str; 2] = ["SPIN_PATH_INFO", "PATH_INFO"];
const MATCHED_ROUTE: [&str; 2] = ["SPIN_MATCHED_ROUTE", "X_MATCHED_ROUTE"];
const COMPONENT_ROUTE: [&str; 2] = ["SPIN_COMPONENT_ROUTE", "X_COMPONENT_ROUTE"];
const RAW_COMPONENT_ROUTE: [&str; 2] = ["SPIN_RAW_COMPONENT_ROUTE", "X_RAW_COMPONENT_ROUTE"];
const BASE_PATH: [&str; 2] = ["SPIN_BASE_PATH", "X_BASE_PATH"];
const CLIENT_ADDR: [&str; 2] = ["SPIN_CLIENT_ADDR", "X_CLIENT_ADDR"];

pub(crate) fn compute_default_headers(
    uri: &Uri,
    base: &str,
    host: &str,
    route_match: &RouteMatch,
    client_addr: SocketAddr,
) -> Result<Vec<([String; 2], String)>> {
    fn owned(strs: &[&'static str; 2]) -> [String; 2] {
        [strs[0].to_owned(), strs[1].to_owned()]
    }

    let owned_full_url: [String; 2] = owned(&FULL_URL);
    let owned_path_info: [String; 2] = owned(&PATH_INFO);
    let owned_matched_route: [String; 2] = owned(&MATCHED_ROUTE);
    let owned_component_route: [String; 2] = owned(&COMPONENT_ROUTE);
    let owned_raw_component_route: [String; 2] = owned(&RAW_COMPONENT_ROUTE);
    let owned_base_path: [String; 2] = owned(&BASE_PATH);
    let owned_client_addr: [String; 2] = owned(&CLIENT_ADDR);

    let mut res = vec![];
    let abs_path = uri
        .path_and_query()
        .expect("cannot get path and query")
        .as_str();

    let path_info = route_match.trailing_wildcard();

    let scheme = uri.scheme_str().unwrap_or("http");

    let full_url = format!("{}://{}{}", scheme, host, abs_path);

    res.push((owned_path_info, path_info));
    res.push((owned_full_url, full_url));
    res.push((owned_matched_route, route_match.based_route().to_string()));

    res.push((owned_base_path, base.to_string()));
    res.push((
        owned_raw_component_route,
        route_match.raw_route().to_string(),
    ));
    res.push((owned_component_route, route_match.raw_route_or_prefix()));
    res.push((owned_client_addr, client_addr.to_string()));

    for (wild_name, wild_value) in route_match.named_wildcards() {
        let wild_header = format!("SPIN_PATH_MATCH_{}", wild_name.to_ascii_uppercase()); // TODO: safer
        let wild_wagi_header = format!("X_PATH_MATCH_{}", wild_name.to_ascii_uppercase()); // TODO: safer
        res.push(([wild_header, wild_wagi_header], wild_value.clone()));
    }

    Ok(res)
}

/// The HTTP executor trait.
/// All HTTP executors must implement this trait.
#[async_trait]
pub(crate) trait HttpExecutor: Clone + Send + Sync + 'static {
    async fn execute(
        &self,
        engine: Arc<TriggerAppEngine<HttpTrigger>>,
        base: &str,
        route_match: &RouteMatch,
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
    // Optional mapping of authority and TLS options for the current component
    client_tls_opts: Option<HashMap<Authority, ParsedClientTlsOpts>>,
    /// The hosts this app is allowed to make outbound requests to
    allowed_hosts: AllowedHostsConfig,
}

impl HttpRuntimeData {
    fn chain_request(
        data: &mut spin_core::Data<Self>,
        request: Request<HyperOutgoingBody>,
        config: wasmtime_wasi_http::types::OutgoingRequestConfig,
        component_id: String,
    ) -> HttpResult<wasmtime_wasi_http::types::HostFutureIncomingResponse> {
        use wasmtime_wasi_http::types::IncomingResponse;

        let this = data.as_ref();

        let chained_handler =
            this.chained_handler
                .clone()
                .ok_or(HttpError::trap(wasmtime::Error::msg(
                    "Internal error: internal request chaining not prepared (engine not assigned)",
                )))?;

        let engine = chained_handler.engine;
        let handler = chained_handler.executor;

        let base = "/";
        let route_match = RouteMatch::synthetic(&component_id, request.uri().path());

        let client_addr = std::net::SocketAddr::from_str("0.0.0.0:0").unwrap();

        let between_bytes_timeout = config.between_bytes_timeout;

        let resp_fut = async move {
            match handler
                .execute(engine.clone(), base, &route_match, request, client_addr)
                .await
            {
                Ok(resp) => Ok(Ok(IncomingResponse {
                    resp,
                    between_bytes_timeout,
                    worker: None,
                })),
                Err(e) => Err(wasmtime::Error::msg(e)),
            }
        };

        let handle = wasmtime_wasi::runtime::spawn(resp_fut);
        Ok(HostFutureIncomingResponse::Pending(handle))
    }
}

fn parse_chaining_target(request: &Request<HyperOutgoingBody>) -> Option<String> {
    parse_service_chaining_target(request.uri())
}

impl OutboundWasiHttpHandler for HttpRuntimeData {
    fn send_request(
        data: &mut spin_core::Data<Self>,
        mut request: Request<HyperOutgoingBody>,
        mut config: wasmtime_wasi_http::types::OutgoingRequestConfig,
    ) -> HttpResult<wasmtime_wasi_http::types::HostFutureIncomingResponse> {
        let this = data.as_mut();

        let is_relative_url = request
            .uri()
            .authority()
            .map(|a| a.host().trim() == "")
            .unwrap_or_default();
        if is_relative_url {
            // Origin must be set in the incoming http handler
            let origin = this.origin.clone().unwrap();
            let path_and_query = request
                .uri()
                .path_and_query()
                .map(|p| p.as_str())
                .unwrap_or("/");
            let uri: Uri = format!("{origin}{path_and_query}")
                .parse()
                // origin together with the path and query must be a valid URI
                .unwrap();
            let host = format!("{}:{}", uri.host().unwrap(), uri.port().unwrap());
            let headers = request.headers_mut();
            headers.insert(
                HOST,
                HeaderValue::from_str(&host).map_err(|_| ErrorCode::HttpProtocolError)?,
            );

            config.use_tls = uri
                .scheme()
                .map(|s| s == &Scheme::HTTPS)
                .unwrap_or_default();
            // We know that `uri` has an authority because we set it above
            *request.uri_mut() = uri;
        }

        let uri = request.uri();
        let uri_string = uri.to_string();
        let unallowed_relative =
            is_relative_url && !this.allowed_hosts.allows_relative_url(&["http", "https"]);
        let unallowed_absolute = !is_relative_url
            && !this.allowed_hosts.allows(
                &OutboundUrl::parse(uri_string, "https")
                    .map_err(|_| ErrorCode::HttpRequestUriInvalid)?,
            );
        if unallowed_relative || unallowed_absolute {
            tracing::error!("Destination not allowed: {}", request.uri());
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
            return Err(ErrorCode::HttpRequestDenied.into());
        }

        if let Some(component_id) = parse_chaining_target(&request) {
            return Self::chain_request(data, request, config, component_id);
        }

        let current_span = tracing::Span::current();
        let uri = request.uri();
        if let Some(authority) = uri.authority() {
            current_span.record("server.address", authority.host());
            if let Some(port) = authority.port() {
                current_span.record("server.port", port.as_u16());
            }
        }

        let client_tls_opts = (data.as_ref()).client_tls_opts.clone();

        // TODO: This is a temporary workaround to make sure that outbound task is instrumented.
        // Once Wasmtime gives us the ability to do the spawn ourselves we can just call .instrument
        // and won't have to do this workaround.
        let response_handle = async move {
            let res = send_request_handler(request, config, client_tls_opts).await;
            if let Ok(res) = &res {
                tracing::Span::current()
                    .record("http.response.status_code", res.resp.status().as_u16());
            }
            Ok(res)
        }
        .in_current_span();
        Ok(HostFutureIncomingResponse::Pending(
            wasmtime_wasi::runtime::spawn(response_handle),
        ))
    }
}

#[derive(Debug, PartialEq)]
enum NotFoundRouteKind {
    Normal(String),
    WellKnown,
}

/// This is a fork of wasmtime_wasi_http::default_send_request_handler function
/// forked from bytecodealliance/wasmtime commit-sha 29a76b68200fcfa69c8fb18ce6c850754279a05b
/// This fork provides the ability to configure client cert auth for mTLS
pub async fn send_request_handler(
    mut request: hyper::Request<HyperOutgoingBody>,
    wasmtime_wasi_http::types::OutgoingRequestConfig {
        use_tls,
        connect_timeout,
        first_byte_timeout,
        between_bytes_timeout,
    }: wasmtime_wasi_http::types::OutgoingRequestConfig,
    client_tls_opts: Option<HashMap<Authority, ParsedClientTlsOpts>>,
) -> Result<wasmtime_wasi_http::types::IncomingResponse, types::ErrorCode> {
    let authority_str = if let Some(authority) = request.uri().authority() {
        if authority.port().is_some() {
            authority.to_string()
        } else {
            let port = if use_tls { 443 } else { 80 };
            format!("{}:{port}", authority)
        }
    } else {
        return Err(types::ErrorCode::HttpRequestUriInvalid);
    };

    let authority = &authority_str.parse::<Authority>().unwrap();

    let tcp_stream = timeout(connect_timeout, TcpStream::connect(&authority_str))
        .await
        .map_err(|_| types::ErrorCode::ConnectionTimeout)?
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::AddrNotAvailable => {
                dns_error("address not available".to_string(), 0)
            }

            _ => {
                if e.to_string()
                    .starts_with("failed to lookup address information")
                {
                    dns_error("address not available".to_string(), 0)
                } else {
                    types::ErrorCode::ConnectionRefused
                }
            }
        })?;

    let (mut sender, worker) = if use_tls {
        #[cfg(any(target_arch = "riscv64", target_arch = "s390x"))]
        {
            return Err(
                wasmtime_wasi_http::bindings::http::types::ErrorCode::InternalError(Some(
                    "unsupported architecture for SSL".to_string(),
                )),
            );
        }

        #[cfg(not(any(target_arch = "riscv64", target_arch = "s390x")))]
        {
            use rustls::pki_types::ServerName;
            let config =
                get_client_tls_config_for_authority(authority, client_tls_opts).map_err(|e| {
                    wasmtime_wasi_http::bindings::http::types::ErrorCode::InternalError(Some(
                        format!(
                            "failed to configure client tls config for authority. error: {}",
                            e
                        ),
                    ))
                })?;
            let connector = tokio_rustls::TlsConnector::from(std::sync::Arc::new(config));
            let mut parts = authority_str.split(':');
            let host = parts.next().unwrap_or(&authority_str);
            let domain = ServerName::try_from(host)
                .map_err(|e| {
                    tracing::warn!("dns lookup error: {e:?}");
                    dns_error("invalid dns name".to_string(), 0)
                })?
                .to_owned();
            let stream = connector.connect(domain, tcp_stream).await.map_err(|e| {
                tracing::warn!("tls protocol error: {e:?}");
                types::ErrorCode::TlsProtocolError
            })?;
            let stream = TokioIo::new(stream);

            let (sender, conn) = timeout(
                connect_timeout,
                hyper::client::conn::http1::handshake(stream),
            )
            .await
            .map_err(|_| types::ErrorCode::ConnectionTimeout)?
            .map_err(hyper_request_error)?;

            let worker = wasmtime_wasi::runtime::spawn(async move {
                match conn.await {
                    Ok(()) => {}
                    // TODO: shouldn't throw away this error and ideally should
                    // surface somewhere.
                    Err(e) => tracing::warn!("dropping error {e}"),
                }
            });

            (sender, worker)
        }
    } else {
        let tcp_stream = TokioIo::new(tcp_stream);
        let (sender, conn) = timeout(
            connect_timeout,
            // TODO: we should plumb the builder through the http context, and use it here
            hyper::client::conn::http1::handshake(tcp_stream),
        )
        .await
        .map_err(|_| types::ErrorCode::ConnectionTimeout)?
        .map_err(hyper_request_error)?;

        let worker = wasmtime_wasi::runtime::spawn(async move {
            match conn.await {
                Ok(()) => {}
                // TODO: same as above, shouldn't throw this error away.
                Err(e) => tracing::warn!("dropping error {e}"),
            }
        });

        (sender, worker)
    };

    // at this point, the request contains the scheme and the authority, but
    // the http packet should only include those if addressing a proxy, so
    // remove them here, since SendRequest::send_request does not do it for us
    *request.uri_mut() = http::Uri::builder()
        .path_and_query(
            request
                .uri()
                .path_and_query()
                .map(|p| p.as_str())
                .unwrap_or("/"),
        )
        .build()
        .expect("comes from valid request");

    let resp = timeout(first_byte_timeout, sender.send_request(request))
        .await
        .map_err(|_| types::ErrorCode::ConnectionReadTimeout)?
        .map_err(hyper_request_error)?
        .map(|body| body.map_err(hyper_request_error).boxed());

    Ok(wasmtime_wasi_http::types::IncomingResponse {
        resp,
        worker: Some(worker),
        between_bytes_timeout,
    })
}

fn get_client_tls_config_for_authority(
    authority: &Authority,
    client_tls_opts: Option<HashMap<Authority, ParsedClientTlsOpts>>,
) -> Result<rustls::ClientConfig> {
    // derived from https://github.com/tokio-rs/tls/blob/master/tokio-rustls/examples/client/src/main.rs
    let ca_webpki_roots = rustls::RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.into(),
    };

    #[allow(clippy::mutable_key_type)]
    let client_tls_opts = match client_tls_opts {
        Some(opts) => opts,
        _ => {
            return Ok(rustls::ClientConfig::builder()
                .with_root_certificates(ca_webpki_roots)
                .with_no_client_auth());
        }
    };

    let client_tls_opts_for_host = match client_tls_opts.get(authority) {
        Some(opts) => opts,
        _ => {
            return Ok(rustls::ClientConfig::builder()
                .with_root_certificates(ca_webpki_roots)
                .with_no_client_auth());
        }
    };

    let mut root_cert_store = if client_tls_opts_for_host.ca_webpki_roots {
        ca_webpki_roots
    } else {
        rustls::RootCertStore::empty()
    };

    if let Some(custom_root_ca) = &client_tls_opts_for_host.custom_root_ca {
        for cer in custom_root_ca {
            match root_cert_store.add(cer.to_owned()) {
                Ok(_) => {}
                Err(e) => {
                    return Err(anyhow::anyhow!(
                        "failed to add custom cert to root_cert_store. error: {}",
                        e
                    ));
                }
            }
        }
    }

    match (
        &client_tls_opts_for_host.cert_chain,
        &client_tls_opts_for_host.private_key,
    ) {
        (Some(cert_chain), Some(private_key)) => Ok(rustls::ClientConfig::builder()
            .with_root_certificates(root_cert_store)
            .with_client_auth_cert(cert_chain.to_owned(), private_key.clone_key())?),
        _ => Ok(rustls::ClientConfig::builder()
            .with_root_certificates(root_cert_store)
            .with_no_client_auth()),
    }
}

/// Translate a [`hyper::Error`] to a wasi-http `ErrorCode` in the context of a request.
pub fn hyper_request_error(err: hyper::Error) -> ErrorCode {
    // If there's a source, we might be able to extract a wasi-http error from it.
    if let Some(cause) = err.source() {
        if let Some(err) = cause.downcast_ref::<ErrorCode>() {
            return err.clone();
        }
    }

    tracing::warn!("hyper request error: {err:?}");

    ErrorCode::HttpProtocolError
}

pub fn dns_error(rcode: String, info_code: u16) -> ErrorCode {
    ErrorCode::DnsError(wasmtime_wasi_http::bindings::http::types::DnsErrorPayload {
        rcode: Some(rcode),
        info_code: Some(info_code),
    })
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

        let (router, _) = Router::build(base, [("DUMMY", &trigger_route.into())])?;
        let route_match = router.route("/base/foo/bar")?;

        let default_headers =
            crate::compute_default_headers(req.uri(), base, host, &route_match, client_addr)?;

        assert_eq!(
            search(&FULL_URL, &default_headers).unwrap(),
            "https://fermyon.dev/base/foo/bar?key1=value1&key2=value2".to_string()
        );
        assert_eq!(
            search(&PATH_INFO, &default_headers).unwrap(),
            "/bar".to_string()
        );
        assert_eq!(
            search(&MATCHED_ROUTE, &default_headers).unwrap(),
            "/base/foo/...".to_string()
        );
        assert_eq!(
            search(&BASE_PATH, &default_headers).unwrap(),
            "/base".to_string()
        );
        assert_eq!(
            search(&RAW_COMPONENT_ROUTE, &default_headers).unwrap(),
            "/foo/...".to_string()
        );
        assert_eq!(
            search(&COMPONENT_ROUTE, &default_headers).unwrap(),
            "/foo".to_string()
        );
        assert_eq!(
            search(&CLIENT_ADDR, &default_headers).unwrap(),
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

        let (router, _) = Router::build(base, [("DUMMY", &trigger_route.into())])?;
        let route_match = router.route("/foo/bar")?;

        let default_headers =
            crate::compute_default_headers(req.uri(), base, host, &route_match, client_addr)?;

        // TODO: we currently replace the scheme with HTTP. When TLS is supported, this should be fixed.
        assert_eq!(
            search(&FULL_URL, &default_headers).unwrap(),
            "https://fermyon.dev/foo/bar?key1=value1&key2=value2".to_string()
        );
        assert_eq!(
            search(&PATH_INFO, &default_headers).unwrap(),
            "/bar".to_string()
        );
        assert_eq!(
            search(&MATCHED_ROUTE, &default_headers).unwrap(),
            "/foo/...".to_string()
        );
        assert_eq!(
            search(&BASE_PATH, &default_headers).unwrap(),
            "/".to_string()
        );
        assert_eq!(
            search(&RAW_COMPONENT_ROUTE, &default_headers).unwrap(),
            "/foo/...".to_string()
        );
        assert_eq!(
            search(&COMPONENT_ROUTE, &default_headers).unwrap(),
            "/foo".to_string()
        );
        assert_eq!(
            search(&CLIENT_ADDR, &default_headers).unwrap(),
            "127.0.0.1:8777".to_string()
        );

        Ok(())
    }

    #[test]
    fn test_default_headers_with_named_wildcards() -> Result<()> {
        let scheme = "https";
        let host = "fermyon.dev";
        let base = "/";
        let trigger_route = "/foo/:userid/...";
        let component_path = "/foo";
        let path_info = "/bar";
        let client_addr: SocketAddr = "127.0.0.1:8777".parse().unwrap();

        let req_uri = format!(
            "{}://{}{}/42{}?key1=value1&key2=value2",
            scheme, host, component_path, path_info
        );

        let req = http::Request::builder()
            .method("POST")
            .uri(req_uri)
            .body("")?;

        let (router, _) = Router::build(base, [("DUMMY", &trigger_route.into())])?;
        let route_match = router.route("/foo/42/bar")?;

        let default_headers =
            crate::compute_default_headers(req.uri(), base, host, &route_match, client_addr)?;

        // TODO: we currently replace the scheme with HTTP. When TLS is supported, this should be fixed.
        assert_eq!(
            search(&FULL_URL, &default_headers).unwrap(),
            "https://fermyon.dev/foo/42/bar?key1=value1&key2=value2".to_string()
        );
        assert_eq!(
            search(&PATH_INFO, &default_headers).unwrap(),
            "/bar".to_string()
        );
        assert_eq!(
            search(&MATCHED_ROUTE, &default_headers).unwrap(),
            "/foo/:userid/...".to_string()
        );
        assert_eq!(
            search(&BASE_PATH, &default_headers).unwrap(),
            "/".to_string()
        );
        assert_eq!(
            search(&RAW_COMPONENT_ROUTE, &default_headers).unwrap(),
            "/foo/:userid/...".to_string()
        );
        assert_eq!(
            search(&COMPONENT_ROUTE, &default_headers).unwrap(),
            "/foo/:userid".to_string()
        );
        assert_eq!(
            search(&CLIENT_ADDR, &default_headers).unwrap(),
            "127.0.0.1:8777".to_string()
        );

        assert_eq!(
            search(
                &["SPIN_PATH_MATCH_USERID", "X_PATH_MATCH_USERID"],
                &default_headers
            )
            .unwrap(),
            "42".to_string()
        );

        Ok(())
    }

    fn search(keys: &[&str; 2], headers: &[([String; 2], String)]) -> Option<String> {
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
