use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use spin_factor_outbound_http::{HostFutureIncomingResponse, SelfRequestOrigin};
use spin_http::routes::RouteMatch;
use spin_outbound_networking::parse_service_chaining_target;
use wasmtime_wasi_http::types::IncomingResponse;

use crate::server::HttpServer;

pub struct OutboundHttpInterceptor {
    server: Arc<HttpServer>,
    origin: SelfRequestOrigin,
}

impl OutboundHttpInterceptor {
    pub fn new(server: Arc<HttpServer>, origin: SelfRequestOrigin) -> Self {
        Self { server, origin }
    }
}

const CHAINED_CLIENT_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0);

impl spin_factor_outbound_http::OutboundHttpInterceptor for OutboundHttpInterceptor {
    fn intercept(
        &self,
        intercepted: spin_factor_outbound_http::Intercepted,
    ) -> Option<wasmtime_wasi_http::HttpResult<spin_factor_outbound_http::HostFutureIncomingResponse>>
    {
        let uri = intercepted.request.uri();

        // Handle service chaining requests
        if let Some(component_id) = parse_service_chaining_target(uri) {
            // TODO: look at the rest of chain_request
            let route_match = RouteMatch::synthetic(&component_id, uri.path());
            let req = std::mem::take(intercepted.request);
            let between_bytes_timeout = intercepted.config.between_bytes_timeout;
            let server = self.server.clone();
            let resp_fut = async move {
                match server
                    .handle_trigger_route(req, route_match, CHAINED_CLIENT_ADDR)
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
            Some(Ok(resp))
        } else {
            intercepted
                .request
                .extensions_mut()
                .insert(self.origin.clone());
            None
        }
    }
}
