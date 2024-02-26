use tracing::Level;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{filter::Targets, prelude::*};

mod metrics;
mod traces;

pub use traces::handle_request;

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
    service: ServiceDescription,
    otel_endpoint: Option<impl Into<String>>,
) -> anyhow::Result<ShutdownGuard> {
    let otel_tracing_layer = match otel_endpoint {
        Some(endpoint) => {
            let endpoint = endpoint.into();
            metrics::init_otel(endpoint.clone())?;
            Some(traces::otel_tracing_layer(service, endpoint)?)
        }
        None => None,
    };

    tracing_subscriber::registry()
        .with(otel_tracing_layer)
        .with(chatty_crates_filter())
        .init();

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

fn chatty_crates_filter() -> Targets {
    Targets::new()
        .with_target("sqlx", Level::WARN)
        .with_default(Level::TRACE)
}
