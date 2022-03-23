use crate::{routes::RoutePattern, ExecutionContext, HttpExecutor};
use anyhow::Result;
use async_trait::async_trait;
use hyper::{body, Body, Request, Response};
use spin_config::WagiConfig;
use spin_engine::io::{IoStreamRedirects, OutRedirect};
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
        let (mut store, instance) = engine.prepare_component(
            component,
            None,
            Some(iostream.clone()),
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

        let log_result = engine.save_output_to_logs(iostream.clone(), component, false, true);

        // Defer checking for failures until here so that the logging runs
        // even if the guest code fails. (And when checking, check the guest
        // result first, so that guest failures are returned in preference to
        // log failures.)
        guest_result??;
        log_result?;

        wagi::handlers::compose_response(iostream.stdout.lock)
    }
}

impl WagiHttpExecutor {
    fn streams_from_body(body: Vec<u8>) -> IoStreamRedirects {
        let stdin = ReadPipe::from(body);
        let stdout_buf = vec![];
        let lock = Arc::new(RwLock::new(stdout_buf));
        let stdout = WritePipe::from_shared(lock.clone());
        let stdout = OutRedirect { out: stdout, lock };

        let stderr_buf = vec![];
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
