use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use http::uri::Scheme;
use spin_factor_outbound_http::{
    HostFutureIncomingResponse, InterceptOutcome, OutgoingRequestConfig, Request,
};
use spin_factors::RuntimeFactors;
use spin_http::routes::RouteMatch;
use spin_outbound_networking::parse_service_chaining_target;
use wasmtime_wasi_http::types::IncomingResponse;

use crate::HttpServer;

/// An outbound HTTP interceptor that handles service chaining requests.
pub struct OutboundHttpInterceptor<F: RuntimeFactors> {
    server: Arc<HttpServer<F>>,
}

impl<F: RuntimeFactors> OutboundHttpInterceptor<F> {
    pub fn new(server: Arc<HttpServer<F>>) -> Self {
        Self { server }
    }
}

const CHAINED_CLIENT_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0);

impl<F: RuntimeFactors> spin_factor_outbound_http::OutboundHttpInterceptor
    for OutboundHttpInterceptor<F>
{
    fn intercept(
        &self,
        request: &mut Request,
        config: &mut OutgoingRequestConfig,
    ) -> InterceptOutcome {
        let uri = request.uri();

        // Handle service chaining requests
        if let Some(component_id) = parse_service_chaining_target(uri) {
            // TODO: look at the rest of chain_request
            let route_match = RouteMatch::synthetic(&component_id, uri.path());
            let req = std::mem::take(request);
            let between_bytes_timeout = config.between_bytes_timeout;
            let server = self.server.clone();
            let resp_fut = async move {
                match server
                    .handle_trigger_route(req, route_match, Scheme::HTTP, CHAINED_CLIENT_ADDR)
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
            let resp = HostFutureIncomingResponse::pending(wasmtime_wasi::runtime::spawn(resp_fut));
            InterceptOutcome::Complete(Ok(resp))
        } else {
            InterceptOutcome::Continue
        }
    }
}
