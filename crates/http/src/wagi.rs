use crate::routes::RoutePattern;
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
        base: &str,
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
        // TODO
        // The default host and TLS fields are currently hard-coded.
        let mut headers = wagi::http_util::build_headers(
            &wagi::dispatcher::RoutePattern::parse(&RoutePattern::sanitize_with_base(
                base, raw_route,
            )),
            &parts,
            len,
            client_addr,
            "default_host",
            false,
            &HashMap::new(),
        );

        // TODO
        // Is there any scenario where the server doesn't populate the host header?
        let default_host = http::HeaderValue::from_str("localhost")?;
        let host = std::str::from_utf8(
            parts
                .headers
                .get("host")
                .unwrap_or(&default_host)
                .as_bytes(),
        )?;
        // Add the default Spin headers.
        // Note that this overrides any existing headers previously set by Wagi.
        for (k, v) in crate::default_headers(&parts.uri, raw_route, base, host)? {
            headers.insert(k, v);
        }

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
