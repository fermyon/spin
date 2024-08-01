use std::{collections::HashMap, future::Future, io::IsTerminal, net::SocketAddr, sync::Arc};

use http::{uri::Scheme, Request, Response, StatusCode, Uri};
use http_body_util::BodyExt;
use hyper::{
    body::{Bytes, Incoming},
    server::conn::http1,
    service::service_fn,
};
use hyper_util::rt::TokioIo;
use spin_app::{APP_DESCRIPTION_KEY, APP_NAME_KEY};
use spin_http::{
    app_info::AppInfo,
    body,
    config::{HttpExecutorType, HttpTriggerConfig},
    routes::{RouteMatch, Router},
};
use spin_outbound_networking::is_service_chaining_host;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpListener,
    task,
};
use tracing::Instrument;
use wasmtime_wasi_http::body::{HyperIncomingBody as Body, HyperOutgoingBody};

use crate::{
    handler::{HandlerType, HttpHandlerExecutor},
    instrument::{finalize_http_span, http_span, instrument_error, MatchedRoute},
    wagi::WagiHttpExecutor,
    NotFoundRouteKind, TlsConfig, TriggerApp, TriggerInstanceBuilder,
};

pub struct HttpServer {
    listen_addr: SocketAddr,
    trigger_app: TriggerApp,
    router: Router,
    // Component ID -> component trigger config
    component_trigger_configs: HashMap<String, HttpTriggerConfig>,
    // Component ID -> handler type
    component_handler_types: HashMap<String, HandlerType>,
}

impl HttpServer {
    pub fn new(
        listen_addr: SocketAddr,
        trigger_app: TriggerApp,
        router: Router,
        component_trigger_configs: HashMap<String, HttpTriggerConfig>,
    ) -> anyhow::Result<Self> {
        let component_handler_types = component_trigger_configs
            .keys()
            .map(|component_id| {
                let component = trigger_app.get_component(component_id)?;
                let handler_type = HandlerType::from_component(trigger_app.engine(), component)?;
                Ok((component_id.clone(), handler_type))
            })
            .collect::<anyhow::Result<_>>()?;
        Ok(Self {
            listen_addr,
            trigger_app,
            router,
            component_trigger_configs,
            component_handler_types,
        })
    }

    pub async fn serve(self: Arc<Self>, listener: TcpListener) -> anyhow::Result<()> {
        self.print_startup_msgs("http", &listener)?;
        loop {
            let (stream, client_addr) = listener.accept().await?;
            self.clone().serve_connection(stream, client_addr);
        }
    }

    pub async fn serve_tls(
        self: Arc<Self>,
        listener: TcpListener,
        tls_config: TlsConfig,
    ) -> anyhow::Result<()> {
        self.print_startup_msgs("https", &listener)?;
        let acceptor = tls_config.server_config()?;
        loop {
            let (stream, client_addr) = listener.accept().await?;
            match acceptor.accept(stream).await {
                Ok(stream) => self.clone().serve_connection(stream, client_addr),
                Err(err) => tracing::error!(?err, "Failed to start TLS session"),
            }
        }
    }

    /// Handles incoming requests using an HTTP executor.
    pub async fn handle(
        &self,
        mut req: Request<Body>,
        scheme: Scheme,
        client_addr: SocketAddr,
    ) -> anyhow::Result<Response<Body>> {
        set_req_uri(&mut req, scheme, self.listen_addr)?;
        strip_forbidden_headers(&mut req);

        spin_telemetry::extract_trace_context(&req);

        tracing::info!("Processing request on URI {}", req.uri());

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

        let app_id = self
            .trigger_app
            .app()
            .get_metadata(APP_NAME_KEY)?
            .unwrap_or_else(|| "<unnamed>".into());

        // Route to app component
        match self.router.route(&path) {
            Ok(route_match) => {
                let component_id = route_match.component_id();

                spin_telemetry::metrics::monotonic_counter!(
                    spin.request_count = 1,
                    trigger_type = "http",
                    app_id = app_id,
                    component_id = component_id
                );

                let instance_builder = self.trigger_app.prepare(component_id)?;
                let trigger_config = self.component_trigger_configs.get(component_id).unwrap();
                let handler_type = self.component_handler_types.get(component_id).unwrap();
                let executor = trigger_config
                    .executor
                    .as_ref()
                    .unwrap_or(&HttpExecutorType::Http);

                let res = match executor {
                    HttpExecutorType::Http => {
                        HttpHandlerExecutor {
                            handler_type: *handler_type,
                        }
                        .execute(instance_builder, &route_match, req, client_addr)
                        .await
                    }
                    HttpExecutorType::Wagi(wagi_config) => {
                        let executor = WagiHttpExecutor {
                            wagi_config: wagi_config.clone(),
                        };
                        executor
                            .execute(instance_builder, &route_match, req, client_addr)
                            .await
                    }
                };
                match res {
                    Ok(res) => Ok(MatchedRoute::with_response_extension(
                        res,
                        route_match.raw_route(),
                    )),
                    Err(err) => {
                        tracing::error!("Error processing request: {err:?}");
                        instrument_error(&err);
                        Self::internal_error(None, route_match.raw_route())
                    }
                }
            }
            Err(_) => Self::not_found(NotFoundRouteKind::Normal(path.to_string())),
        }
    }

    /// Returns spin status information.
    fn app_info(&self, route: String) -> anyhow::Result<Response<Body>> {
        let info = AppInfo::new(self.trigger_app.app());
        let body = serde_json::to_vec_pretty(&info)?;
        Ok(MatchedRoute::with_response_extension(
            Response::builder()
                .header("content-type", "application/json")
                .body(body::full(body.into()))?,
            route,
        ))
    }

    /// Creates an HTTP 500 response.
    fn internal_error(
        body: Option<&str>,
        route: impl Into<String>,
    ) -> anyhow::Result<Response<Body>> {
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
    fn not_found(kind: NotFoundRouteKind) -> anyhow::Result<Response<Body>> {
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
        client_addr: SocketAddr,
    ) {
        task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .keep_alive(true)
                .serve_connection(
                    TokioIo::new(stream),
                    service_fn(move |request| {
                        self.clone().instrumented_service_fn(client_addr, request)
                    }),
                )
                .await
            {
                tracing::warn!("Error serving HTTP connection: {err:?}");
            }
        });
    }

    async fn instrumented_service_fn(
        self: Arc<Self>,
        client_addr: SocketAddr,
        request: Request<Incoming>,
    ) -> anyhow::Result<Response<HyperOutgoingBody>> {
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
                    client_addr,
                )
                .await;
            finalize_http_span(result, method)
        }
        .instrument(span)
        .await
    }

    fn print_startup_msgs(&self, scheme: &str, listener: &TcpListener) -> anyhow::Result<()> {
        let local_addr = listener.local_addr()?;
        let base_url = format!("{scheme}://{local_addr:?}");
        terminal::step!("\nServing", "{base_url}");
        tracing::info!("Serving {base_url}");

        println!("Available Routes:");
        for (route, component_id) in self.router.routes() {
            println!("  {}: {}{}", component_id, base_url, route);
            if let Some(component) = self.trigger_app.app().get_component(component_id) {
                if let Some(description) = component.get_metadata(APP_DESCRIPTION_KEY)? {
                    println!("    {}", description);
                }
            }
        }
        Ok(())
    }
}

/// The incoming request's scheme and authority
///
/// The incoming request's URI is relative to the server, so we need to set the scheme and authority
fn set_req_uri(req: &mut Request<Body>, scheme: Scheme, addr: SocketAddr) -> anyhow::Result<()> {
    let uri = req.uri().clone();
    let mut parts = uri.into_parts();
    let authority = format!("{}:{}", addr.ip(), addr.port()).parse().unwrap();
    parts.scheme = Some(scheme);
    parts.authority = Some(authority);
    *req.uri_mut() = Uri::from_parts(parts).unwrap();
    Ok(())
}

pub fn strip_forbidden_headers(req: &mut Request<Body>) {
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
pub const FULL_URL: [&str; 2] = ["SPIN_FULL_URL", "X_FULL_URL"];
pub const PATH_INFO: [&str; 2] = ["SPIN_PATH_INFO", "PATH_INFO"];
pub const MATCHED_ROUTE: [&str; 2] = ["SPIN_MATCHED_ROUTE", "X_MATCHED_ROUTE"];
pub const COMPONENT_ROUTE: [&str; 2] = ["SPIN_COMPONENT_ROUTE", "X_COMPONENT_ROUTE"];
pub const RAW_COMPONENT_ROUTE: [&str; 2] = ["SPIN_RAW_COMPONENT_ROUTE", "X_RAW_COMPONENT_ROUTE"];
pub const BASE_PATH: [&str; 2] = ["SPIN_BASE_PATH", "X_BASE_PATH"];
pub const CLIENT_ADDR: [&str; 2] = ["SPIN_CLIENT_ADDR", "X_CLIENT_ADDR"];

pub(crate) fn compute_default_headers(
    uri: &Uri,
    host: &str,
    route_match: &RouteMatch,
    client_addr: SocketAddr,
) -> anyhow::Result<Vec<([String; 2], String)>> {
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

    res.push((owned_base_path, "/".to_string()));
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

/// An HTTP executor.
pub(crate) trait HttpExecutor: Clone + Send + Sync + 'static {
    fn execute(
        &self,
        instance_builder: TriggerInstanceBuilder,
        route_match: &RouteMatch,
        req: Request<Body>,
        client_addr: SocketAddr,
    ) -> impl Future<Output = anyhow::Result<Response<Body>>>;
}
