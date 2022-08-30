mod util;

use std::net::SocketAddr;

use anyhow::Result;
use async_trait::async_trait;
use bytes::Buf;
use hyper::{body, Body, Request, Response};
use serde::{Deserialize, Serialize};
use spin_trigger::TriggerAppEngine;
use tracing::log;

use crate::{routes::RoutePattern, HttpExecutor, HttpTrigger};

/// Wagi specific configuration for the http executor.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WagiTriggerConfig {
    /// The name of the entrypoint.
    #[serde(default)]
    pub entrypoint: String,

    /// A string representation of the argv array.
    ///
    /// This should be a space-separate list of strings. The value
    /// ${SCRIPT_NAME} will be replaced with the Wagi SCRIPT_NAME,
    /// and the value ${ARGS} will be replaced with the query parameter
    /// name/value pairs presented as args. For example,
    /// `param1=val1&param2=val2` will become `param1=val1 param2=val2`,
    /// which will then be presented to the program as two arguments
    /// in argv.
    #[serde(default)]
    pub argv: String,
}

impl Default for WagiTriggerConfig {
    fn default() -> Self {
        /// This is the default Wagi entrypoint.
        const WAGI_DEFAULT_ENTRYPOINT: &str = "_start";
        const WAGI_DEFAULT_ARGV: &str = "${SCRIPT_NAME} ${ARGS}";

        Self {
            entrypoint: WAGI_DEFAULT_ENTRYPOINT.to_owned(),
            argv: WAGI_DEFAULT_ARGV.to_owned(),
        }
    }
}

#[derive(Clone)]
pub struct WagiHttpExecutor {
    pub wagi_config: WagiTriggerConfig,
}

#[async_trait]
impl HttpExecutor for WagiHttpExecutor {
    async fn execute(
        &self,
        app_engine: &TriggerAppEngine<HttpTrigger>,
        component: &str,
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
        let body = body::aggregate(body).await?;
        let content_length = body.remaining();
        // TODO
        // The default host and TLS fields are currently hard-coded.
        let mut headers = util::build_headers(
            &RoutePattern::from(raw_route),
            &parts,
            content_length,
            client_addr,
            "default_host",
            false,
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
        for (keys, val) in crate::compute_default_headers(&parts.uri, raw_route, host)? {
            headers.insert(keys[1].to_string(), val);
        }

        let mut store_builder = app_engine.store_builder(component)?;

        // Set up Wagi environment
        store_builder.args(argv.split(' '))?;
        store_builder.env(headers)?;
        store_builder.stdin_pipe(body.reader());
        let mut stdout_buffer = store_builder.stdout_buffered();

        let (instance, mut store) = app_engine
            .prepare_instance_with_store(component, store_builder)
            .await?;

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
        let guest_result = start.call_async(&mut store, &[], &mut []).await;
        tracing::info!("Module execution complete");
        guest_result.or_else(ignore_successful_proc_exit_trap)?;

        let stdout = stdout_buffer.take();
        util::compose_response(&stdout)
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
