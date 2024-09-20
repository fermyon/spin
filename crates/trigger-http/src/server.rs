use std::{collections::HashMap, future::Future, io::IsTerminal, net::SocketAddr, sync::Arc};

use anyhow::{bail, Context};
use http::{
    uri::{Authority, Scheme},
    Request, Response, StatusCode, Uri,
};
use http_body_util::BodyExt;
use hyper::{
    body::{Bytes, Incoming},
    server::conn::http1,
    service::service_fn,
};
use hyper_util::rt::TokioIo;
use spin_app::{APP_DESCRIPTION_KEY, APP_NAME_KEY};
use spin_factor_outbound_http::{OutboundHttpFactor, SelfRequestOrigin};
use spin_factors::RuntimeFactors;
use spin_http::{
    app_info::AppInfo,
    body,
    config::{HttpExecutorType, HttpTriggerConfig},
    routes::{RouteMatch, Router},
};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpListener,
    task,
};
use tracing::Instrument;
use wasmtime::component::Component;
use wasmtime_wasi_http::body::HyperOutgoingBody;

use crate::{
    headers::strip_forbidden_headers,
    instrument::{finalize_http_span, http_span, instrument_error, MatchedRoute},
    outbound_http::OutboundHttpInterceptor,
    spin::SpinHttpExecutor,
    wagi::WagiHttpExecutor,
    wasi::WasiHttpExecutor,
    Body, NotFoundRouteKind, TlsConfig, TriggerApp, TriggerInstanceBuilder,
};

/// An HTTP server which runs Spin apps.
pub struct HttpServer<F: RuntimeFactors> {
    /// The address the server is listening on.
    listen_addr: SocketAddr,
    /// The TLS configuration for the server.
    tls_config: Option<TlsConfig>,
    /// Request router.
    router: Router,
    /// The app being triggered.
    trigger_app: TriggerApp<F>,
    // Component ID -> component trigger config
    component_trigger_configs: HashMap<String, HttpTriggerConfig>,
    // Component ID -> handler type
    component_handler_types: HashMap<String, HandlerType>,
}

impl<F: RuntimeFactors> HttpServer<F> {
    /// Create a new [`HttpServer`].
    pub fn new(
        listen_addr: SocketAddr,
        tls_config: Option<TlsConfig>,
        trigger_app: TriggerApp<F>,
    ) -> anyhow::Result<Self> {
        // This needs to be a vec before building the router to handle duplicate routes
        let component_trigger_configs = Vec::from_iter(
            trigger_app
                .app()
                .trigger_configs::<HttpTriggerConfig>("http")?
                .into_iter()
                .map(|(_, config)| (config.component.clone(), config)),
        );

        // Build router
        let component_routes = component_trigger_configs
            .iter()
            .map(|(component_id, config)| (component_id.as_str(), &config.route));
        let (router, duplicate_routes) = Router::build("/", component_routes)?;
        if !duplicate_routes.is_empty() {
            tracing::error!(
                "The following component routes are duplicates and will never be used:"
            );
            for dup in &duplicate_routes {
                tracing::error!(
                    "  {}: {} (duplicate of {})",
                    dup.replaced_id,
                    dup.route(),
                    dup.effective_id,
                );
            }
        }
        tracing::trace!(
            "Constructed router: {:?}",
            router.routes().collect::<Vec<_>>()
        );

        // Now that router is built we can merge duplicate routes by component
        let component_trigger_configs = HashMap::from_iter(component_trigger_configs);

        let component_handler_types = component_trigger_configs
            .iter()
            .map(|(component_id, trigger_config)| {
                let handler_type = match &trigger_config.executor {
                    None | Some(HttpExecutorType::Http) => {
                        let component = trigger_app.get_component(component_id)?;
                        HandlerType::from_component(trigger_app.engine(), component)?
                    }
                    Some(HttpExecutorType::Wagi(wagi_config)) => {
                        anyhow::ensure!(
                            wagi_config.entrypoint == "_start",
                            "Wagi component '{component_id}' cannot use deprecated 'entrypoint' field"
                        );
                        HandlerType::Wagi
                    }
                };
                Ok((component_id.clone(), handler_type))
            })
            .collect::<anyhow::Result<_>>()?;
        Ok(Self {
            listen_addr,
            tls_config,
            router,
            trigger_app,
            component_trigger_configs,
            component_handler_types,
        })
    }

    /// Serve incoming requests over the provided [`TcpListener`].
    pub async fn serve(self: Arc<Self>) -> anyhow::Result<()> {
        let listener = TcpListener::bind(self.listen_addr).await.with_context(|| {
            format!(
                "Unable to listen on {listen_addr}",
                listen_addr = self.listen_addr
            )
        })?;
        if let Some(tls_config) = self.tls_config.clone() {
            self.serve_https(listener, tls_config).await?;
        } else {
            self.serve_http(listener).await?;
        }
        Ok(())
    }

    async fn serve_http(self: Arc<Self>, listener: TcpListener) -> anyhow::Result<()> {
        self.print_startup_msgs("http", &listener)?;
        loop {
            let (stream, client_addr) = listener.accept().await?;
            self.clone()
                .serve_connection(stream, Scheme::HTTP, client_addr);
        }
    }

    async fn serve_https(
        self: Arc<Self>,
        listener: TcpListener,
        tls_config: TlsConfig,
    ) -> anyhow::Result<()> {
        self.print_startup_msgs("https", &listener)?;
        let acceptor = tls_config.server_config()?;
        loop {
            let (stream, client_addr) = listener.accept().await?;
            match acceptor.accept(stream).await {
                Ok(stream) => self
                    .clone()
                    .serve_connection(stream, Scheme::HTTPS, client_addr),
                Err(err) => tracing::error!(?err, "Failed to start TLS session"),
            }
        }
    }

    /// Handles incoming requests using an HTTP executor.
    ///
    /// This method handles well known paths and routes requests to the handler when the router
    /// matches the requests path.
    pub async fn handle(
        self: &Arc<Self>,
        mut req: Request<Body>,
        server_scheme: Scheme,
        client_addr: SocketAddr,
    ) -> anyhow::Result<Response<Body>> {
        strip_forbidden_headers(&mut req);

        spin_telemetry::extract_trace_context(&req);

        let path = req.uri().path().to_string();

        tracing::info!("Processing request on path '{path}'");

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

        match self.router.route(&path) {
            Ok(route_match) => {
                self.handle_trigger_route(req, route_match, server_scheme, client_addr)
                    .await
            }
            Err(_) => Self::not_found(NotFoundRouteKind::Normal(path.to_string())),
        }
    }

    /// Handles a successful route match.
    pub async fn handle_trigger_route(
        self: &Arc<Self>,
        mut req: Request<Body>,
        route_match: RouteMatch,
        server_scheme: Scheme,
        client_addr: SocketAddr,
    ) -> anyhow::Result<Response<Body>> {
        set_req_uri(&mut req, server_scheme.clone())?;
        let app_id = self
            .trigger_app
            .app()
            .get_metadata(APP_NAME_KEY)?
            .unwrap_or_else(|| "<unnamed>".into());

        let component_id = route_match.component_id();

        spin_telemetry::metrics::monotonic_counter!(
            spin.request_count = 1,
            trigger_type = "http",
            app_id = app_id,
            component_id = component_id
        );

        let mut instance_builder = self.trigger_app.prepare(component_id)?;

        // Set up outbound HTTP request origin and service chaining
        // The outbound HTTP factor is required since both inbound and outbound wasi HTTP
        // implementations assume they use the same underlying wasmtime resource storage.
        // Eventually, we may be able to factor this out to a separate factor.
        let outbound_http = instance_builder
            .factor_builder::<OutboundHttpFactor>()
            .context(
            "The wasi HTTP trigger was configured without the required wasi outbound http support",
        )?;
        let origin = SelfRequestOrigin::create(server_scheme, &self.listen_addr.to_string())?;
        outbound_http.set_self_request_origin(origin);
        outbound_http.set_request_interceptor(OutboundHttpInterceptor::new(self.clone()))?;

        // Prepare HTTP executor
        let trigger_config = self.component_trigger_configs.get(component_id).unwrap();
        let handler_type = self.component_handler_types.get(component_id).unwrap();
        let executor = trigger_config
            .executor
            .as_ref()
            .unwrap_or(&HttpExecutorType::Http);

        let res = match executor {
            HttpExecutorType::Http => match handler_type {
                HandlerType::Spin => {
                    SpinHttpExecutor
                        .execute(instance_builder, &route_match, req, client_addr)
                        .await
                }
                HandlerType::Wasi0_2
                | HandlerType::Wasi2023_11_10
                | HandlerType::Wasi2023_10_18 => {
                    WasiHttpExecutor {
                        handler_type: *handler_type,
                    }
                    .execute(instance_builder, &route_match, req, client_addr)
                    .await
                }
                HandlerType::Wagi => unreachable!(),
            },
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
        server_scheme: Scheme,
        client_addr: SocketAddr,
    ) {
        task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .keep_alive(true)
                .serve_connection(
                    TokioIo::new(stream),
                    service_fn(move |request| {
                        self.clone().instrumented_service_fn(
                            server_scheme.clone(),
                            client_addr,
                            request,
                        )
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
        server_scheme: Scheme,
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
                    server_scheme,
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
/// The incoming request's URI is relative to the server, so we need to set the scheme and authority.
/// Either the `Host` header or the request's URI's authority is used as the source of truth for the authority.
/// This function will error if the authority cannot be unambiguously determined.
fn set_req_uri(req: &mut Request<Body>, scheme: Scheme) -> anyhow::Result<()> {
    let uri = req.uri().clone();
    let mut parts = uri.into_parts();
    let headers = req.headers();
    let header_authority = headers
        .get(http::header::HOST)
        .map(|h| -> anyhow::Result<Authority> {
            let host_header = h.to_str().context("'Host' header is not valid UTF-8")?;
            host_header
                .parse()
                .context("'Host' header contains an invalid authority")
        })
        .transpose()?;
    let uri_authority = parts.authority;

    // Get authority either from request URI or from 'Host' header
    let authority = match (header_authority, uri_authority) {
        (None, None) => bail!("no 'Host' header present in request"),
        (None, Some(a)) => a,
        (Some(a), None) => a,
        (Some(a1), Some(a2)) => {
            // Ensure that if `req.authority` is set, it matches what was in the `Host` header
            // https://github.com/hyperium/hyper/issues/1612
            if a1 != a2 {
                return Err(anyhow::anyhow!(
                    "authority in 'Host' header does not match authority in URI"
                ));
            }
            a1
        }
    };
    parts.scheme = Some(scheme);
    parts.authority = Some(authority);
    *req.uri_mut() = Uri::from_parts(parts).unwrap();
    Ok(())
}

/// An HTTP executor.
pub(crate) trait HttpExecutor: Clone + Send + Sync + 'static {
    fn execute<F: RuntimeFactors>(
        &self,
        instance_builder: TriggerInstanceBuilder<F>,
        route_match: &RouteMatch,
        req: Request<Body>,
        client_addr: SocketAddr,
    ) -> impl Future<Output = anyhow::Result<Response<Body>>>;
}

/// Whether this handler uses the custom Spin http handler interface for wasi-http
#[derive(Clone, Copy)]
pub enum HandlerType {
    Spin,
    Wagi,
    Wasi0_2,
    Wasi2023_11_10,
    Wasi2023_10_18,
}

pub const WASI_HTTP_EXPORT_2023_10_18: &str = "wasi:http/incoming-handler@0.2.0-rc-2023-10-18";
pub const WASI_HTTP_EXPORT_2023_11_10: &str = "wasi:http/incoming-handler@0.2.0-rc-2023-11-10";
pub const WASI_HTTP_EXPORT_0_2_0: &str = "wasi:http/incoming-handler@0.2.0";
pub const WASI_HTTP_EXPORT_0_2_1: &str = "wasi:http/incoming-handler@0.2.1";

impl HandlerType {
    /// Determine the handler type from the exports of a component
    pub fn from_component(
        engine: impl AsRef<wasmtime::Engine>,
        component: &Component,
    ) -> anyhow::Result<HandlerType> {
        let mut handler_ty = None;

        let mut set = |ty: HandlerType| {
            if handler_ty.is_none() {
                handler_ty = Some(ty);
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "component exports multiple different handlers but \
                     it's expected to export only one"
                ))
            }
        };
        let ty = component.component_type();
        for (name, _) in ty.exports(engine.as_ref()) {
            match name {
                WASI_HTTP_EXPORT_2023_10_18 => set(HandlerType::Wasi2023_10_18)?,
                WASI_HTTP_EXPORT_2023_11_10 => set(HandlerType::Wasi2023_11_10)?,
                WASI_HTTP_EXPORT_0_2_0 | WASI_HTTP_EXPORT_0_2_1 => set(HandlerType::Wasi0_2)?,
                "fermyon:spin/inbound-http" => set(HandlerType::Spin)?,
                _ => {}
            }
        }

        handler_ty.ok_or_else(|| {
            anyhow::anyhow!(
                "Expected component to export one of \
                `{WASI_HTTP_EXPORT_2023_10_18}`, \
                `{WASI_HTTP_EXPORT_2023_11_10}`, \
                `{WASI_HTTP_EXPORT_0_2_0}`, \
                `{WASI_HTTP_EXPORT_0_2_1}`, \
                 or `fermyon:spin/inbound-http` but it exported none of those"
            )
        })
    }
}
