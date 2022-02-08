use crate::spin_http::{Method, SpinHttp, SpinHttpData};
use crate::HttpExecutor;
use crate::{ExecutionContext, RuntimeContext};
use anyhow::Result;
use async_trait::async_trait;
use http::StatusCode;
use hyper::{
    server::conn::AddrStream,
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use std::{collections::HashMap, net::SocketAddr, str::FromStr, sync::Arc};
use tracing::{instrument, log};
use url::Url;
use wasmtime::{Instance, Store};

#[derive(Clone)]
pub struct WagiHttpExecutor;

#[async_trait]
impl HttpExecutor for WagiHttpExecutor {
    #[instrument(skip(engine))]
    async fn execute(
        engine: &ExecutionContext,
        component: String,
        req: Request<Body>,
    ) -> Result<Response<Body>> {
        log::info!(
            "Executing request using the Wagi executor for component {}",
            component
        );
        let (store, instance) = engine.prepare_component(component, None)?;

        todo!("Wagi executor not implemented yet.")
    }
}
