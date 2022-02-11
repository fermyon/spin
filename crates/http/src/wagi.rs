use crate::ExecutionContext;
use crate::HttpExecutor;
use anyhow::Result;
use async_trait::async_trait;
use hyper::{body, Body, Request, Response};
use spin_engine::io::{IoStreamRedirects, OutRedirect};
use std::collections::HashMap;
use std::{
    net::SocketAddr,
    sync::{Arc, RwLock},
};
use tracing::log;
use wasi_common::pipe::{ReadPipe, WritePipe};

/// This is the default Wagi entrypoint.
/// There should be a way to set this in the component
/// configuration of the trigger / executor.
const WAGI_DEFAULT_ENTRYPOINT: &str = "_start";

#[derive(Clone)]
pub struct WagiHttpExecutor;

#[async_trait]
impl HttpExecutor for WagiHttpExecutor {
    async fn execute(
        engine: &ExecutionContext,
        component: &str,
        raw_route: &str,
        req: Request<Body>,
        client_addr: SocketAddr,
    ) -> Result<Response<Body>> {
        log::trace!(
            "Executing request using the Wagi executor for component {}",
            component
        );

        let (parts, body) = req.into_parts();
        let body = body::to_bytes(body).await?.to_vec();
        let len = body.len();
        let iostream = Self::streams_from_body(body);
        let headers = wagi::http_util::build_headers(
            &wagi::dispatcher::RoutePattern::parse(raw_route),
            &parts,
            len,
            client_addr,
            "default_host",
            false,
            &HashMap::new(),
        );

        let (mut store, instance) =
            engine.prepare_component(component, None, Some(iostream.clone()), Some(headers))?;

        let start = instance
            .get_func(&mut store, WAGI_DEFAULT_ENTRYPOINT)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No such function '{}' in {}",
                    WAGI_DEFAULT_ENTRYPOINT,
                    component
                )
            })?;
        tracing::trace!("Calling Wasm entry point");
        start.call(&mut store, &[], &mut [])?;
        tracing::trace!("Module execution complete");

        wagi::handlers::compose_response(iostream.stdout.lock)
    }
}

impl WagiHttpExecutor {
    fn streams_from_body(body: Vec<u8>) -> IoStreamRedirects {
        let stdin = ReadPipe::from(body);
        let stdout_buf: Vec<u8> = vec![];
        let lock = Arc::new(RwLock::new(stdout_buf));
        let stdout = WritePipe::from_shared(lock.clone());
        let stdout = OutRedirect { out: stdout, lock };

        let stderr_buf: Vec<u8> = vec![];
        let lock = Arc::new(RwLock::new(stderr_buf));
        let stderr = WritePipe::from_shared(lock.clone());
        let stderr = OutRedirect { out: stderr, lock };

        IoStreamRedirects {
            stdin,
            stdout,
            stderr,
        }
    }
}
