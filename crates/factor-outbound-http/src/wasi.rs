use std::{error::Error, sync::Arc};

use anyhow::Context;
use http::{header::HOST, uri::Authority, Request, Uri};
use http_body_util::BodyExt;
use rustls::ClientConfig;
use spin_factor_outbound_networking::{OutboundAllowedHosts, OutboundUrl};
use spin_factors::{wasmtime::component::ResourceTable, RuntimeFactorsInstanceState};
use tokio::{net::TcpStream, time::timeout};
use tracing::Instrument;
use wasmtime_wasi_http::{
    bindings::http::types::ErrorCode,
    body::HyperOutgoingBody,
    io::TokioIo,
    types::{HostFutureIncomingResponse, IncomingResponse},
    WasiHttpCtx, WasiHttpImpl, WasiHttpView,
};

use crate::{
    wasi_2023_10_18, wasi_2023_11_10, InstanceState, Intercepted, OutboundHttpFactor,
    SelfRequestOrigin,
};

pub(crate) fn add_to_linker<T: Send + 'static>(
    ctx: &mut spin_factors::InitContext<T, OutboundHttpFactor>,
) -> anyhow::Result<()> {
    fn type_annotate<T, F>(f: F) -> F
    where
        F: Fn(&mut T) -> WasiHttpImpl<WasiHttpImplInner>,
    {
        f
    }
    let get_data_with_table = ctx.get_data_with_table_fn();
    let closure = type_annotate(move |data| {
        let (state, table) = get_data_with_table(data);
        WasiHttpImpl(WasiHttpImplInner { state, table })
    });
    let linker = ctx.linker();
    wasmtime_wasi_http::bindings::http::outgoing_handler::add_to_linker_get_host(linker, closure)?;
    wasmtime_wasi_http::bindings::http::types::add_to_linker_get_host(linker, closure)?;

    wasi_2023_10_18::add_to_linker(linker, closure)?;
    wasi_2023_11_10::add_to_linker(linker, closure)?;

    Ok(())
}

impl OutboundHttpFactor {
    pub fn get_wasi_http_impl(
        runtime_instance_state: &mut impl RuntimeFactorsInstanceState,
    ) -> Option<WasiHttpImpl<impl WasiHttpView + '_>> {
        let (state, table) = runtime_instance_state.get_with_table::<OutboundHttpFactor>()?;
        Some(WasiHttpImpl(WasiHttpImplInner { state, table }))
    }
}

pub(crate) struct WasiHttpImplInner<'a> {
    state: &'a mut InstanceState,
    table: &'a mut ResourceTable,
}

impl<'a> WasiHttpView for WasiHttpImplInner<'a> {
    fn ctx(&mut self) -> &mut WasiHttpCtx {
        &mut self.state.wasi_http_ctx
    }

    fn table(&mut self) -> &mut ResourceTable {
        self.table
    }

    fn send_request(
        &mut self,
        mut request: Request<wasmtime_wasi_http::body::HyperOutgoingBody>,
        mut config: wasmtime_wasi_http::types::OutgoingRequestConfig,
    ) -> wasmtime_wasi_http::HttpResult<wasmtime_wasi_http::types::HostFutureIncomingResponse> {
        if let Some(interceptor) = &self.state.request_interceptor {
            let intercepted = Intercepted {
                request: &mut request,
                config: &mut config,
            };
            if let Some(res) = interceptor.intercept(intercepted) {
                return res;
            }
        }

        let host = request.uri().host().unwrap_or_default();
        let tls_client_config = self
            .state
            .component_tls_configs
            .get_client_config(host)
            .clone();

        Ok(HostFutureIncomingResponse::Pending(
            wasmtime_wasi::runtime::spawn(
                send_request_impl(
                    request,
                    config,
                    self.state.allowed_hosts.clone(),
                    tls_client_config,
                )
                .in_current_span(),
            ),
        ))
    }
}

async fn send_request_impl(
    mut request: Request<wasmtime_wasi_http::body::HyperOutgoingBody>,
    mut config: wasmtime_wasi_http::types::OutgoingRequestConfig,
    allowed_hosts: OutboundAllowedHosts,
    tls_client_config: Arc<ClientConfig>,
) -> anyhow::Result<Result<IncomingResponse, ErrorCode>> {
    let allowed_hosts = allowed_hosts.resolve().await?;

    let is_relative_url = request.uri().authority().is_none();
    if is_relative_url {
        if !allowed_hosts.allows_relative_url(&["http", "https"]) {
            return handle_not_allowed(request.uri(), true);
        }

        let origin = request
            .extensions()
            .get::<SelfRequestOrigin>()
            .cloned()
            .context("cannot send relative outbound request; no 'origin' set by host")?;

        config.use_tls = origin.use_tls();

        request.headers_mut().insert(HOST, origin.host_header());

        let path_and_query = request.uri().path_and_query().cloned();
        *request.uri_mut() = origin.into_uri(path_and_query);
    } else {
        let outbound_url = OutboundUrl::parse(request.uri().to_string(), "https")
            .map_err(|_| ErrorCode::HttpRequestUriInvalid)?;
        if !allowed_hosts.allows(&outbound_url) {
            return handle_not_allowed(request.uri(), false);
        }
    }

    if let Some(authority) = request.uri().authority() {
        let current_span = tracing::Span::current();
        current_span.record("server.address", authority.host());
        if let Some(port) = authority.port() {
            current_span.record("server.port", port.as_u16());
        }
    }

    Ok(send_request_handler(request, config, tls_client_config).await)
}

// TODO(factors): Move to some callback on spin-factor-outbound-networking (?)
fn handle_not_allowed(
    uri: &Uri,
    is_relative: bool,
) -> anyhow::Result<Result<IncomingResponse, ErrorCode>> {
    tracing::error!("Destination not allowed: {uri}");
    let host = if is_relative {
        terminal::warn!("A component tried to make a HTTP request to the same component but it does not have permission.");
        "self".to_string()
    } else {
        let host = format!(
            "{scheme}://{authority}",
            scheme = uri.scheme_str().unwrap_or_default(),
            authority = uri.authority().map(Authority::as_str).unwrap_or_default()
        );
        terminal::warn!("A component tried to make a HTTP request to non-allowed host '{host}'.");
        host
    };
    eprintln!("To allow requests, add 'allowed_outbound_hosts = [\"{host}\"]' to the manifest component section.");
    Err(ErrorCode::HttpRequestDenied.into())
}

/// This is a fork of wasmtime_wasi_http::default_send_request_handler function
/// forked from bytecodealliance/wasmtime commit-sha 29a76b68200fcfa69c8fb18ce6c850754279a05b
/// This fork provides the ability to configure client cert auth for mTLS
async fn send_request_handler(
    mut request: http::Request<HyperOutgoingBody>,
    wasmtime_wasi_http::types::OutgoingRequestConfig {
        use_tls,
        connect_timeout,
        first_byte_timeout,
        between_bytes_timeout,
    }: wasmtime_wasi_http::types::OutgoingRequestConfig,
    tls_client_config: Arc<ClientConfig>,
) -> Result<wasmtime_wasi_http::types::IncomingResponse, ErrorCode> {
    let authority_str = if let Some(authority) = request.uri().authority() {
        if authority.port().is_some() {
            authority.to_string()
        } else {
            let port = if use_tls { 443 } else { 80 };
            format!("{}:{port}", authority)
        }
    } else {
        return Err(ErrorCode::HttpRequestUriInvalid);
    };

    let tcp_stream = timeout(connect_timeout, TcpStream::connect(&authority_str))
        .await
        .map_err(|_| ErrorCode::ConnectionTimeout)?
        .map_err(|err| match err.kind() {
            std::io::ErrorKind::AddrNotAvailable => {
                dns_error("address not available".to_string(), 0)
            }
            _ => {
                if err
                    .to_string()
                    .starts_with("failed to lookup address information")
                {
                    dns_error("address not available".to_string(), 0)
                } else {
                    ErrorCode::ConnectionRefused
                }
            }
        })?;

    let (mut sender, worker) = if use_tls {
        #[cfg(any(target_arch = "riscv64", target_arch = "s390x"))]
        {
            return Err(ErrorCode::InternalError(Some(
                "unsupported architecture for SSL".to_string(),
            )));
        }

        #[cfg(not(any(target_arch = "riscv64", target_arch = "s390x")))]
        {
            use rustls::pki_types::ServerName;
            let connector = tokio_rustls::TlsConnector::from(tls_client_config);
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
                ErrorCode::TlsProtocolError
            })?;
            let stream = TokioIo::new(stream);

            let (sender, conn) = timeout(
                connect_timeout,
                hyper::client::conn::http1::handshake(stream),
            )
            .await
            .map_err(|_| ErrorCode::ConnectionTimeout)?
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
        .map_err(|_| ErrorCode::ConnectionTimeout)?
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
        .map_err(|_| ErrorCode::ConnectionReadTimeout)?
        .map_err(hyper_request_error)?
        .map(|body| body.map_err(hyper_request_error).boxed());

    Ok(wasmtime_wasi_http::types::IncomingResponse {
        resp,
        worker: Some(worker),
        between_bytes_timeout,
    })
}

/// Translate a [`hyper::Error`] to a wasi-http `ErrorCode` in the context of a request.
fn hyper_request_error(err: hyper::Error) -> ErrorCode {
    // If there's a source, we might be able to extract a wasi-http error from it.
    if let Some(cause) = err.source() {
        if let Some(err) = cause.downcast_ref::<ErrorCode>() {
            return err.clone();
        }
    }

    tracing::warn!("hyper request error: {err:?}");

    ErrorCode::HttpProtocolError
}

fn dns_error(rcode: String, info_code: u16) -> ErrorCode {
    ErrorCode::DnsError(wasmtime_wasi_http::bindings::http::types::DnsErrorPayload {
        rcode: Some(rcode),
        info_code: Some(info_code),
    })
}
