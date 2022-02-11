use crate::ExecutionContext;
use crate::HttpExecutor;
use anyhow::Result;
use async_trait::async_trait;
use hyper::{Body, Request, Response};
use std::net::SocketAddr;
use tracing::log;

#[derive(Clone)]
pub struct WagiHttpExecutor;

#[async_trait]
impl HttpExecutor for WagiHttpExecutor {
    async fn execute(
        _engine: &ExecutionContext,
        _component: &str,
        _req: Request<Body>,
        _client_addr: SocketAddr,
    ) -> Result<Response<Body>> {
        log::trace!(
            "Executing request using the Wagi executor for component {}",
            _component
        );

        todo!("Wagi executor not implemented yet.")
    }
}
