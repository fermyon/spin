use std::cell::Cell;
use std::io::IsTerminal;
use std::time::Duration;
use std::time::Instant;

use env::otel_logs_enabled;
use env::otel_metrics_enabled;
use env::otel_tracing_enabled;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use tracing_subscriber::{fmt, prelude::*, registry, EnvFilter, Layer};

pub mod detector;
mod env;
pub mod logs;
pub mod metrics;
mod propagation;
mod traces;

pub use propagation::extract_trace_context;
pub use propagation::inject_trace_context;

/// Initializes telemetry for Spin using the [tracing] library.
///
/// Under the hood this involves initializing a [tracing::Subscriber] with multiple [Layer]s. One
/// [Layer] emits [tracing] events to stderr, another sends spans to an OTel collector, and another
/// sends metrics to an OTel collector.
///
/// Configuration for the OTel layers is pulled from the environment.
///
/// Examples of emitting traces from Spin:
///
/// ```no_run
/// # use tracing::instrument;
/// # use tracing::Level;
/// #[instrument(name = "span_name", err(level = Level::INFO), fields(otel.name = "dynamically set name"))]
/// fn func_you_want_to_trace() -> anyhow::Result<String> {
///     Ok("Hello, world!".to_string())
/// }
/// ```
///
/// Some notes on tracing:
///
/// - If you don't want the span to be collected by default emit it at a trace or debug level.
/// - Make sure you `.in_current_span()` any spawned tasks so the span context is propagated.
/// - Use the otel.name attribute to dynamically set the span name.
/// - Use the err argument to have instrument automatically handle errors.
///
/// Examples of emitting metrics from Spin:
///
/// ```no_run
/// spin_telemetry::metrics::monotonic_counter!(spin.metric_name = 1, metric_attribute = "value");
/// ```
pub fn init(spin_version: String) -> anyhow::Result<ShutdownGuard> {
    // This layer will print all tracing library log messages to stderr.
    let fmt_layer = fmt::layer()
        .with_writer(std::io::stderr)
        .with_ansi(std::io::stderr().is_terminal())
        .with_filter(
            // Filter directives explained here https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html#directives
            EnvFilter::from_default_env()
                // Wasmtime is too noisy
                .add_directive("wasmtime_wasi_http=warn".parse()?)
                // Watchexec is too noisy
                .add_directive("watchexec=off".parse()?)
                // We don't want to duplicate application logs
                .add_directive("[{app_log}]=off".parse()?)
                .add_directive("[{app_log_non_utf8}]=off".parse()?),
        );

    // Even if metrics or tracing aren't enabled we're okay to turn on the global error handler
    opentelemetry::global::set_error_handler(otel_error_handler)?;

    let otel_tracing_layer = if otel_tracing_enabled() {
        Some(traces::otel_tracing_layer(spin_version.clone())?)
    } else {
        None
    };

    let otel_metrics_layer = if otel_metrics_enabled() {
        Some(metrics::otel_metrics_layer(spin_version.clone())?)
    } else {
        None
    };

    // Build a registry subscriber with the layers we want to use.
    registry()
        .with(otel_tracing_layer)
        .with(otel_metrics_layer)
        .with(fmt_layer)
        .init();

    // Used to propagate trace information in the standard W3C TraceContext format. Even if the otel
    // layer is disabled we still want to propagate trace context.
    opentelemetry::global::set_text_map_propagator(TraceContextPropagator::new());

    if otel_logs_enabled() {
        logs::init_otel_logging_backend(spin_version)?;
    }

    Ok(ShutdownGuard)
}

fn otel_error_handler(err: opentelemetry::global::Error) {
    // Track the error count
    let signal = match err {
        opentelemetry::global::Error::Metric(_) => "metrics",
        opentelemetry::global::Error::Trace(_) => "traces",
        opentelemetry::global::Error::Log(_) => "logs",
        _ => "unknown",
    };
    metrics::monotonic_counter!(spin.otel_error_count = 1, signal = signal);

    // Only log the first error at ERROR level, subsequent errors will be logged at higher levels and rate limited
    static FIRST_OTEL_ERROR: std::sync::Once = std::sync::Once::new();
    FIRST_OTEL_ERROR.call_once(|| {
        tracing::error!(?err, "OpenTelemetry error");
        tracing::error!("There has been an error with the OpenTelemetry system. Traces, logs or metrics are likely failing to export.");
        tracing::error!("Further OpenTelemetry errors will be available at WARN level (rate limited) or at TRACE level.");
    });

    // Rate limit the logging of the OTel errors to not occur more frequently on each thread than OTEL_ERROR_INTERVAL
    const OTEL_ERROR_INTERVAL: Duration = Duration::from_millis(5000);
    thread_local! {
        static LAST_OTEL_ERROR: Cell<Instant> = Cell::new(Instant::now() - OTEL_ERROR_INTERVAL);
    }
    if LAST_OTEL_ERROR.get().elapsed() > OTEL_ERROR_INTERVAL {
        LAST_OTEL_ERROR.set(Instant::now());
        tracing::warn!(?err, "OpenTelemetry error");
    } else {
        tracing::trace!(?err, "OpenTelemetry error");
    }
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
