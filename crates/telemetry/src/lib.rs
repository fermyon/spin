use std::io::IsTerminal;

use opentelemetry::sdk::trace::Tracer;
use std::sync::OnceLock;
use tracing::{info, level_filters::LevelFilter};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{
    filter::{FilterFn, Filtered},
    fmt,
    prelude::*,
    registry,
    reload::{self, Handle},
    EnvFilter, Registry,
};
use url::Url;

mod metrics;
mod traces;

pub use traces::handle_request;

static OTEL_LAYER_WACKY_REHANDLE_THING: OnceLock<
    Handle<Option<Filtered<OpenTelemetryLayer<Registry, Tracer>, LevelFilter, Registry>>, Registry>,
> = OnceLock::new();

// TODO: Remove concept of service description

/// Description of the service for which telemetry is being collected
pub struct ServiceDescription {
    name: String,
    version: Option<String>,
}

impl ServiceDescription {
    pub fn new<S1: Into<String>, S2: Into<String>>(name: S1, version: Option<S2>) -> Self {
        Self {
            name: name.into(),
            version: version.map(|s| s.into()),
        }
    }
}

/// Initialize open telemetry
///
/// Sets the open telemetry pipeline as the default tracing subscriber
pub fn init(
    _service: ServiceDescription,
    _otel_endpoint: Option<impl Into<String>>,
) -> anyhow::Result<ShutdownGuard> {
    let fmt_layer = fmt::layer()
        .with_writer(std::io::stderr)
        .with_ansi(std::io::stderr().is_terminal())
        .with_filter(
            EnvFilter::from_default_env()
                .add_directive("wasmtime_wasi_http=warn".parse()?)
                .add_directive("watchexec=off".parse()?),
        );

    let (telemetry_layer, reload_handle) = reload::Layer::new(None);

    registry().with(telemetry_layer).with(fmt_layer).init();

    let r = OTEL_LAYER_WACKY_REHANDLE_THING.set(reload_handle);

    r.is_err().then(|| {
        info!("stuff blew up");

        // return error
        anyhow::anyhow!("stuff blew up")
    });

    Ok(ShutdownGuard(None))
}

pub fn reload(service: ServiceDescription, endpoint: Url) {
    let endpoint = endpoint.to_string();
    let otel_tracing_layer = Some(traces::otel_tracing_layer(service, endpoint).expect("todo"));

    OTEL_LAYER_WACKY_REHANDLE_THING
        .get()
        .unwrap()
        .reload(otel_tracing_layer)
        .expect("this to not fail todo fix");

    info!("reloaded tracing layer");
}

/// An RAII implementation for connection to open telemetry services.
///
/// Shutdown of the open telemetry services will happen on `Drop`.
#[must_use]
pub struct ShutdownGuard(Option<WorkerGuard>);

impl Drop for ShutdownGuard {
    fn drop(&mut self) {
        // Give tracer provider a chance to flush any pending traces.
        opentelemetry::global::shutdown_tracer_provider();
    }
}
