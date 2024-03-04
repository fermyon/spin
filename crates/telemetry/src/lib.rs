use std::io::IsTerminal;

use opentelemetry::sdk::trace::Tracer;
use std::sync::OnceLock;
use tracing::level_filters::LevelFilter;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{
    filter::Filtered,
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

type TelemetryLayer = Filtered<OpenTelemetryLayer<Registry, Tracer>, LevelFilter, Registry>;

static GLOBAL_TELEMETRY_LAYER_RELOAD_HANDLE: OnceLock<Handle<Option<TelemetryLayer>, Registry>> =
    OnceLock::new();

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

/// TODO
///
/// Sets the open telemetry pipeline as the default tracing subscriber
pub fn init_globally() -> anyhow::Result<ShutdownGuard> {
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

    let result = GLOBAL_TELEMETRY_LAYER_RELOAD_HANDLE.set(reload_handle);
    if result.is_err() {
        return Err(anyhow::anyhow!(
            "failed to set global telemetry layer reload handle",
        ));
    }

    Ok(ShutdownGuard(None))
}

/// TODO
pub fn reload_globally(service: ServiceDescription, endpoint: Url, traces: bool, metrics: bool) {
    if traces {
        if let Err(error) = reload_telemetry_layer(service, endpoint) {
            tracing::error!("failed to load otlp telemetry: {}", error);
        }
    }
    if metrics {
        // TODO: Setup metrics
    }
}

/// TODO
fn reload_telemetry_layer(service: ServiceDescription, endpoint: Url) -> anyhow::Result<()> {
    let otel_tracing_layer = Some(traces::otel_tracing_layer(service, endpoint.to_string())?);

    GLOBAL_TELEMETRY_LAYER_RELOAD_HANDLE
        .get()
        .unwrap()
        .reload(otel_tracing_layer)
        .map_err(|e| anyhow::anyhow!(e))
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
