use crate::ExecutionContext;
use crate::HttpExecutor;
use anyhow::Result;
use async_trait::async_trait;
use hyper::{Body, Request, Response};
use std::net::SocketAddr;
use tracing::{instrument, log};

#[derive(Clone)]
pub struct WagiHttpExecutor;

#[async_trait]
impl HttpExecutor for WagiHttpExecutor {
    #[instrument(skip(_engine))]
    async fn execute(
        _engine: &ExecutionContext,
        _component: &String,
        _req: Request<Body>,
        _client_addr: SocketAddr,
    ) -> Result<Response<Body>> {
        log::info!(
            "Executing request using the Wagi executor for component {}",
            _component
        );

        todo!("Wagi executor not implemented yet.")
    }
}
