use std::{collections::HashMap, error::Error as _, str::FromStr};

use http::{
    header,
    uri::{Authority, Scheme},
    HeaderValue, Request, Uri,
};
use hyper_util::rt::TokioIo;
use spin_factor_outbound_networking::OutboundUrl;
use spin_factors::{wasmtime::component::ResourceTable, RuntimeFactorsInstanceState};
use spin_http::routes::RouteMatch;
use tokio::{net::TcpStream, time::timeout};
use tracing::Instrument as _;
use wasmtime_wasi_http::{
    bindings::http::types::{self, ErrorCode},
    body::HyperOutgoingBody,
    types::HostFutureIncomingResponse,
    HttpError, WasiHttpCtx, WasiHttpImpl, WasiHttpView,
};

use crate::{wasi_2023_10_18, wasi_2023_11_10, OutboundHttpFactor};

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
        WasiHttpImpl(WasiHttpImplInner {
            ctx: &mut state.wasi_http_ctx,
            table,
        })
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
        Some(WasiHttpImpl(WasiHttpImplInner {
            ctx: &mut state.wasi_http_ctx,
            table,
        }))
    }
}

pub(crate) struct WasiHttpImplInner<'a> {
    ctx: &'a mut WasiHttpCtx,
    table: &'a mut ResourceTable,
    data: Data,
}

impl<'a> WasiHttpView for WasiHttpImplInner<'a> {
    fn ctx(&mut self) -> &mut WasiHttpCtx {
        self.ctx
    }

    fn table(&mut self) -> &mut ResourceTable {
        self.table
    }

    fn send_request(
        &mut self,
        mut request: Request<wasmtime_wasi_http::body::HyperOutgoingBody>,
        config: wasmtime_wasi_http::types::OutgoingRequestConfig,
    ) -> wasmtime_wasi_http::HttpResult<wasmtime_wasi_http::types::HostFutureIncomingResponse> {
        spin_telemetry::inject_trace_context(&mut request);
        self.handle_request(request, config)
    }
}

impl<'a> WasiHttpImplInner<'a> {
    fn handle_request(
        &self,
        request: Request<wasmtime_wasi_http::body::HyperOutgoingBody>,
        config: wasmtime_wasi_http::types::OutgoingRequestConfig,
    ) -> wasmtime_wasi_http::HttpResult<wasmtime_wasi_http::types::HostFutureIncomingResponse> {
        let is_relative_url = request
            .uri()
            .authority()
            .map(|a| a.host().trim() == "")
            .unwrap_or_default();
        if is_relative_url {
            // Origin must be set in the incoming http handler
            let origin = self.origin.clone().unwrap();
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
                header::HOST,
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
            is_relative_url && !self.allowed_hosts.allows_relative_url(&["http", "https"]);
        let unallowed_absolute = !is_relative_url
            && !self.allowed_hosts.allows(
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
                // terminal::warn!(
                //     "A component tried to make a HTTP request to non-allowed host '{host}'."
                // );
                let scheme = uri.scheme().unwrap_or(&Scheme::HTTPS);
                format!("{scheme}://{host}:{port}")
            } else {
                // terminal::warn!("A component tried to make a HTTP request to the same component but it does not have permission.");
                "self".into()
            };
            // eprintln!("To allow requests, add 'allowed_outbound_hosts = [\"{}\"]' to the manifest component section.", host);
            return Err(ErrorCode::HttpRequestDenied.into());
        }

        if let Some(component_id) = parse_chaining_target(&request) {
            return self.chain_request(request, config, component_id);
        }

        let current_span = tracing::Span::current();
        let uri = request.uri();
        if let Some(authority) = uri.authority() {
            current_span.record("server.address", authority.host());
            if let Some(port) = authority.port() {
                current_span.record("server.port", port.as_u16());
            }
        }

        let client_tls_opts = self.client_tls_opts.clone();

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

impl<'a> WasiHttpImplInner<'a> {
    fn chain_request(
        &self,
        request: Request<HyperOutgoingBody>,
        config: wasmtime_wasi_http::types::OutgoingRequestConfig,
        component_id: String,
    ) -> wasmtime_wasi_http::HttpResult<wasmtime_wasi_http::types::HostFutureIncomingResponse> {
        use wasmtime_wasi_http::types::IncomingResponse;

        let chained_handler =
            self.chained_handler
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
) -> anyhow::Result<rustls::ClientConfig> {
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
