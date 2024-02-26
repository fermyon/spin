use opentelemetry::{
    propagation::{Extractor, TextMapPropagator},
    sdk::{propagation::TraceContextPropagator, trace, Resource},
};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_semantic_conventions::resource::{SERVICE_NAME, SERVICE_VERSION};
use tracing::metadata::LevelFilter;
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::{Layer, Registry};

use super::ServiceDescription;

/// Set current span's parent to context datadog expects
pub fn handle_request<'a>(req: impl Into<RequestExtractor<'a>>) {
    let context = opentelemetry_datadog::DatadogPropagator::new().extract(&req.into());
    tracing::Span::current().set_parent(context);
}

pub(crate) fn otel_tracing_layer(
    service: ServiceDescription,
    endpoint: String,
) -> anyhow::Result<impl Layer<Registry>> {
    opentelemetry::global::set_text_map_propagator(TraceContextPropagator::new());

    let mut service_metadata = vec![SERVICE_NAME.string(service.name)];
    if let Some(version) = service.version {
        service_metadata.push(SERVICE_VERSION.string(version));
    }

    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(endpoint),
        )
        .with_trace_config(trace::config().with_resource(Resource::new(service_metadata)))
        .install_batch(opentelemetry::runtime::Tokio)?;
    Ok(tracing_opentelemetry::layer()
        .with_tracer(tracer)
        .with_threads(false)
        .with_filter(LevelFilter::INFO))
}

pub enum RequestExtractor<'a> {
    Http0(&'a http0::HeaderMap),
    Http1(&'a http1::HeaderMap),
}

impl<'a> Extractor for RequestExtractor<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        match self {
            RequestExtractor::Http0(headers) => {
                headers.get(key).map(|v| v.to_str().unwrap_or_default())
            }
            RequestExtractor::Http1(headers) => {
                headers.get(key).map(|v| v.to_str().unwrap_or_default())
            }
        }
    }

    fn keys(&self) -> Vec<&str> {
        unimplemented!()
    }
}

impl<'a, T> From<&'a http0::Request<T>> for RequestExtractor<'a> {
    fn from(req: &'a http0::Request<T>) -> Self {
        Self::Http0(req.headers())
    }
}

impl<'a, T> From<&'a http1::Request<T>> for RequestExtractor<'a> {
    fn from(req: &'a http1::Request<T>) -> Self {
        Self::Http1(req.headers())
    }
}
