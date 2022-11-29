mod util;

use std::{io::Cursor, net::SocketAddr};

use anyhow::Result;
use async_trait::async_trait;
use hyper::{
    body::{self},
    Body, Request, Response,
};
use serde::{Deserialize, Serialize};
use spin_core::I32Exit;
use spin_trigger::TriggerAppEngine;

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
        engine: &TriggerAppEngine<HttpTrigger>,
        component: &str,
        base: &str,
        raw_route: &str,
        req: Request<Body>,
        client_addr: SocketAddr,
    ) -> Result<Response<Body>> {
        tracing::trace!(
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
        let mut headers = util::build_headers(
            &RoutePattern::from(base, raw_route),
            &parts,
            len,
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
        for (keys, val) in crate::compute_default_headers(&parts.uri, raw_route, base, host)? {
            headers.insert(keys[1].to_string(), val);
        }

        let mut store_builder = engine.store_builder(component)?;

        // Set up Wagi environment
        store_builder.args(argv.split(' '))?;
        store_builder.env(headers)?;
        store_builder.stdin_pipe(Cursor::new(body));
        let mut stdout_buffer = store_builder.stdout_buffered();

        let (instance, mut store) = engine
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
        start
            .call_async(&mut store, &[], &mut [])
            .await
            .or_else(ignore_successful_proc_exit_trap)?;
        tracing::info!("Module execution complete");

        util::compose_response(&stdout_buffer.take())
    }
}

fn ignore_successful_proc_exit_trap(guest_err: anyhow::Error) -> Result<()> {
    match guest_err.root_cause().downcast_ref::<I32Exit>() {
        Some(trap) => match trap.0 {
            0 => Ok(()),
            _ => Err(guest_err),
        },
        None => Err(guest_err),
    }
}
