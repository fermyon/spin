use std::io::IsTerminal;

use config::Config;
use opentelemetry::global;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, prelude::*, registry, EnvFilter};

pub mod config;
mod traces;

pub use traces::accept_trace;

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
pub fn init(service: ServiceDescription, config: Config) -> anyhow::Result<ShutdownGuard> {
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
    let otlp_layer = if config.is_enabled {
        Some(traces::otel_tracing_layer(
            service,
            config
                .otel_exporter_otlp_traces_endpoint
                .unwrap()
                .to_string(),
        )?)
    } else {
        None
    };

    // TODO: comment
    registry().with(otlp_layer).with(fmt_layer).init();

    // TODO: comment
    global::set_text_map_propagator(TraceContextPropagator::new());

    Ok(ShutdownGuard(None))
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
