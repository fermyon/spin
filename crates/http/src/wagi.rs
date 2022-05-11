use crate::{routes::RoutePattern, ExecutionContext, HttpExecutor};
use anyhow::Result;
use async_trait::async_trait;
use hyper::{body, Body, Request, Response};
use spin_engine::io::{
    redirect_to_mem_buffer, Follow, ModuleIoRedirects, OutputBuffers, WriteDestinations,
};
use spin_manifest::WagiConfig;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, RwLock, RwLockReadGuard},
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
        follow: bool,
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
        let (redirects, outputs) = Self::streams_from_body(body, follow);
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

        let (mut store, instance) = engine.prepare_component(
            component,
            None,
            Some(redirects),
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
        let guest_result = spawn_blocking(move || start.call(&mut store, &[], &mut [])).await;
        tracing::info!("Module execution complete");

        let log_result = engine.save_output_to_logs(outputs.read(), component, false, true);

        // Defer checking for failures until here so that the logging runs
        // even if the guest code fails. (And when checking, check the guest
        // result first, so that guest failures are returned in preference to
        // log failures.)
        guest_result?.or_else(ignore_successful_proc_exit_trap)?;
        log_result?;

        wagi::handlers::compose_response(outputs.stdout)
    }
}

impl WagiHttpExecutor {
    fn streams_from_body(
        body: Vec<u8>,
        follow_on_stderr: bool,
    ) -> (ModuleIoRedirects, WagiRedirectReadHandles) {
        let stdin = ReadPipe::from(body);

        let stdout_buf = vec![];
        let stdout_lock = Arc::new(RwLock::new(stdout_buf));
        let stdout_pipe = WritePipe::from_shared(stdout_lock.clone());

        let (stderr_pipe, stderr_lock) = redirect_to_mem_buffer(Follow::stderr(follow_on_stderr), None);

        let rd = ModuleIoRedirects::new(
            Box::new(stdin),
            Box::new(stdout_pipe),
            stderr_pipe,
        );

        let h = WagiRedirectReadHandles {
            stdout: stdout_lock,
            stderr: stderr_lock,
        };

        (rd, h)
    }
}

struct WagiRedirectReadHandles {
    stdout: Arc<RwLock<Vec<u8>>>,
    stderr: Arc<RwLock<WriteDestinations>>,
}

impl WagiRedirectReadHandles {
    fn read(&self) -> impl OutputBuffers + '_ {
        WagiRedirectReadHandlesLock {
            stdout: self.stdout.read().unwrap(),
            stderr: self.stderr.read().unwrap(),
        }
    }
}

struct WagiRedirectReadHandlesLock<'a> {
    stdout: RwLockReadGuard<'a, Vec<u8>>,
    stderr: RwLockReadGuard<'a, WriteDestinations>,
}

impl<'a> OutputBuffers for WagiRedirectReadHandlesLock<'a> {
    fn stdout(&self) -> &[u8] {
        &self.stdout
    }
    fn stderr(&self) -> &[u8] {
        self.stderr.buffer()
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
