use std::io::IsTerminal;

use config::Config;
use opentelemetry::global;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, prelude::*, registry, EnvFilter, Layer};

pub mod config;
mod traces;

pub use traces::extract_trace_context;
pub use traces::inject_trace_context;

/// Initializes telemetry for Spin using the [tracing] library.
///
/// Under the hood this involves initializing a [tracing::Subscriber] with multiple [Layer]s. One
/// [Layer] emits [tracing] events to stderr, and another sends spans to an OTLP compliant
/// collector.
pub fn init(config: Config) -> anyhow::Result<ShutdownGuard> {
    // This layer will print all tracing library log messages to stderr.
    let fmt_layer = fmt::layer()
        .with_writer(std::io::stderr)
        .with_ansi(std::io::stderr().is_terminal())
        .with_filter(
            EnvFilter::from_default_env()
                .add_directive("wasmtime_wasi_http=warn".parse()?)
                .add_directive("watchexec=off".parse()?),
        );

    let otlp_layer = if config.otel_sdk_disabled {
        None
    } else {
        Some(traces::otlp_tracing_layer(config)?)
    };

    // Build a registry subscriber with the layers we want to use.
    registry().with(otlp_layer).with(fmt_layer).init();

    // Used to propagate trace information in the standard W3C TraceContext format.
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
