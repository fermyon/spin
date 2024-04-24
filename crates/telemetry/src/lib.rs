use std::io::IsTerminal;

use env::otel_metrics_enabled;
use env::otel_tracing_enabled;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use tracing_subscriber::{fmt, prelude::*, registry, EnvFilter, Layer};

pub mod detector;
mod env;
pub mod metrics;
mod propagation;
mod traces;

pub use propagation::extract_trace_context;
pub use propagation::inject_trace_context;

/// Initializes telemetry for Spin using the [tracing] library.
///
/// Under the hood this involves initializing a [tracing::Subscriber] with multiple [Layer]s. One
/// [Layer] emits [tracing] events to stderr, and another sends spans to an OTEL collector.
///
/// Configuration is pulled from the environment.
pub fn init(spin_version: String) -> anyhow::Result<ShutdownGuard> {
    // This layer will print all tracing library log messages to stderr.
    let fmt_layer = fmt::layer()
        .with_writer(std::io::stderr)
        .with_ansi(std::io::stderr().is_terminal())
        .with_filter(
            EnvFilter::from_default_env()
                .add_directive("wasmtime_wasi_http=warn".parse()?)
                .add_directive("watchexec=off".parse()?),
        );

    // Even if metrics or tracing aren't enabled we're okay to turn on the global error handler
    opentelemetry::global::set_error_handler(otel_error_handler)?;

    if otel_metrics_enabled() {
        metrics::init(spin_version.clone())?;
    }

    let otel_layer = if otel_tracing_enabled() {
        Some(traces::otel_tracing_layer(spin_version)?)
    } else {
        None
    };

    // Build a registry subscriber with the layers we want to use.
    registry().with(otel_layer).with(fmt_layer).init();

    // Used to propagate trace information in the standard W3C TraceContext format. Even if the otel
    // layer is disabled we still want to propagate trace context.
    opentelemetry::global::set_text_map_propagator(TraceContextPropagator::new());

    Ok(ShutdownGuard)
}

fn otel_error_handler(err: opentelemetry::global::Error) {
    static FIRST_OTEL_ERROR: std::sync::Once = std::sync::Once::new();
    FIRST_OTEL_ERROR.call_once(|| {
        tracing::error!("There has been an error with the OpenTelemetry system, traces and metrics are likely failing to export");
        tracing::error!("Further OpenTelemetry errors will be logged at DEBUG level")
    });
    tracing::debug!(?err, "OpenTelemetry error");
}

/// An RAII implementation for connection to open telemetry services.
///
/// Shutdown of the open telemetry services will happen on `Drop`.
#[must_use]
pub struct ShutdownGuard;

impl Drop for ShutdownGuard {
    fn drop(&mut self) {
        // Give tracer provider a chance to flush any pending traces.
        opentelemetry::global::shutdown_tracer_provider();
    }
}
