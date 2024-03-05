use std::io::IsTerminal;

use opentelemetry::{
    global,
    sdk::{propagation::TraceContextPropagator, trace::Tracer},
};
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

pub use traces::accept_trace;

type TelemetryLayer = Filtered<OpenTelemetryLayer<Registry, Tracer>, LevelFilter, Registry>;

static GLOBAL_TELEMETRY_LAYER_RELOAD_HANDLE: OnceLock<Handle<Option<TelemetryLayer>, Registry>> =
    OnceLock::new();
static GLOBAL_SERVICE_DESCRIPTION: OnceLock<ServiceDescription> = OnceLock::new();

/// Description of the service for which telemetry is being collected
#[derive(Clone)]
pub struct ServiceDescription {
    name: String,
    version: String,
}

impl ServiceDescription {
    pub fn new<S1: Into<String>, S2: Into<String>>(name: S1, version: S2) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
        }
    }
}

/// TODO
///
/// Sets the open telemetry pipeline as the default tracing subscriber
pub fn init_globally(service: ServiceDescription) -> anyhow::Result<ShutdownGuard> {
    // Globally set the service description
    let result = GLOBAL_SERVICE_DESCRIPTION.set(service);
    if result.is_err() {
        return Err(anyhow::anyhow!("failed to set global service description",));
    }

    // TODO: comment
    let fmt_layer = fmt::layer()
        .with_writer(std::io::stderr)
        .with_ansi(std::io::stderr().is_terminal())
        .with_filter(
            EnvFilter::from_default_env()
                .add_directive("wasmtime_wasi_http=warn".parse()?)
                .add_directive("watchexec=off".parse()?),
        );

    // TODO: comment
    let (telemetry_layer, reload_handle) = reload::Layer::new(None);

    // TODO: comment
    let result = GLOBAL_TELEMETRY_LAYER_RELOAD_HANDLE.set(reload_handle);
    if result.is_err() {
        return Err(anyhow::anyhow!(
            "failed to set global telemetry layer reload handle",
        ));
    }

    // TODO: comment
    registry().with(telemetry_layer).with(fmt_layer).init();

    // TODO: comment
    global::set_text_map_propagator(TraceContextPropagator::new());

    Ok(ShutdownGuard(None))
}

/// TODO
pub fn reload_globally(endpoint: Url, traces: bool, metrics: bool) {
    if traces {
        if let Err(error) = reload_telemetry_layer(endpoint) {
            tracing::error!("failed to load otlp telemetry: {}", error);
        }
    }
    if metrics {
        // TODO: Setup metrics
    }
}

/// TODO
fn reload_telemetry_layer(endpoint: Url) -> anyhow::Result<()> {
    let service = GLOBAL_SERVICE_DESCRIPTION.get().unwrap();
    let otel_tracing_layer = Some(traces::otel_tracing_layer(
        service.clone(),
        endpoint.to_string(),
    )?);

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
