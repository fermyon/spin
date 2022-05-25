use crate::{routes::RoutePattern, ExecutionContext, HttpExecutor};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use hyper::{body, Body, Request, Response};
use spin_engine::io::ComponentStdioOverrides;
use spin_manifest::WagiConfig;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, RwLock},
};
use tokio::task::spawn_blocking;
use tracing::log;
use wasi_common::pipe::{ReadPipe, WritePipe};

#[derive(Clone)]
pub struct WagiHttpExecutor {
    pub wagi_config: WagiConfig,
}

#[async_trait]
impl HttpExecutor for WagiHttpExecutor {
    async fn execute(
        &self,
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

        let uri_path = req.uri().path();

        // Build the argv array by starting with the config for `argv` and substituting in
        // script name and args where appropriate.
        let script_name = uri_path.to_string();
        let args = req.uri().query().unwrap_or_default().replace('&', " ");
        let argv = self
            .wagi_config
            .argv
            .clone()
            .replace("${SCRIPT_NAME}", &script_name)
            .replace("${ARGS}", &args);

        let (parts, body) = req.into_parts();

        let body = body::to_bytes(body).await?.to_vec();
        let len = body.len();
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

        let default_host = http::HeaderValue::from_str("localhost")?;
        let host = std::str::from_utf8(
            parts
                .headers
                .get("host")
                .unwrap_or(&default_host)
                .as_bytes(),
        )?;

        // Add the default Spin headers.
        // This sets the current environment variables Wagi expects (such as
        // `PATH_INFO`, or `X_FULL_URL`).
        // Note that this overrides any existing headers previously set by Wagi.
        for (keys, val) in crate::compute_default_headers(&parts.uri, raw_route, base, host)? {
            headers.insert(keys[1].to_string(), val);
        }

        let stdin = ReadPipe::from(body);
        let stdout = WritePipe::new_in_memory();
        let stdio_overrides = ComponentStdioOverrides::default()
            .stdin(stdin)
            .stdout(stdout.clone());

        let (mut store, instance) = engine.prepare_component(
            component,
            None,
            stdio_overrides,
            Some(headers),
            Some(argv.split(' ').map(|s| s.to_owned()).collect()),
        )?;

        let start = instance
            .get_func(&mut store, &self.wagi_config.entrypoint)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "No such function '{}' in {}",
                    self.wagi_config.entrypoint,
                    component
                )
            })?;
        tracing::trace!("Calling Wasm entry point");
        spawn_blocking(move || start.call(&mut store, &[], &mut []))
            .await?
            .or_else(ignore_successful_proc_exit_trap)?;

        let stdout_bytes = stdout
            .try_into_inner()
            .map_err(|_| anyhow!("stdout WritePipe has multiple refs!"))?
            .into_inner();
        wagi::handlers::compose_response(Arc::new(RwLock::new(stdout_bytes)))
    }
}

fn ignore_successful_proc_exit_trap(guest_err: anyhow::Error) -> Result<()> {
    match guest_err.root_cause().downcast_ref::<wasmtime::Trap>() {
        Some(trap) => match trap.i32_exit_status() {
            Some(0) => Ok(()),
            _ => Err(guest_err),
        },
        None => Err(guest_err),
    }
}
