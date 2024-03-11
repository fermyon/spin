use opentelemetry::{
    global,
    propagation::{Extractor, Injector},
    KeyValue,
};
use opentelemetry_otlp::{ExportConfig, Protocol, SpanExporterBuilder, WithExportConfig};
use opentelemetry_sdk::{
    trace::{config, Tracer},
    Resource,
};
use opentelemetry_semantic_conventions::resource::{SERVICE_NAME, SERVICE_VERSION};
use tracing::metadata::LevelFilter;
use tracing_opentelemetry::{OpenTelemetryLayer, OpenTelemetrySpanExt};
use tracing_subscriber::{Layer, Registry};

use super::ServiceDescription;

/// Constructs a layer for the tracing subscriber that sends spans to an OTLP compliant collector.
pub(crate) fn otlp_tracing_layer(
    service: ServiceDescription,
    tracing_config: ExportConfig,
) -> anyhow::Result<
    tracing_subscriber::filter::Filtered<
        OpenTelemetryLayer<Registry, Tracer>,
        LevelFilter,
        Registry,
    >,
> {
    let service_metadata = vec![
        KeyValue::new(SERVICE_NAME, service.name),
        KeyValue::new(SERVICE_VERSION, service.version),
    ];

    let exporter: SpanExporterBuilder = match tracing_config.protocol {
        Protocol::Grpc => opentelemetry_otlp::new_exporter()
            .tonic()
            .with_export_config(tracing_config)
            .into(),
        Protocol::HttpBinary => opentelemetry_otlp::new_exporter()
            .http()
            .with_export_config(tracing_config)
            .into(),
    };

    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(config().with_resource(Resource::new(service_metadata)))
        .install_batch(opentelemetry_sdk::runtime::Tokio)?;
    Ok(tracing_opentelemetry::layer()
        .with_tracer(tracer)
        .with_threads(false)
        .with_filter(LevelFilter::INFO))
}

/// Injects the current OTEL trace context into the provided request.
pub fn inject_trace_context<'a>(req: impl Into<HeaderInjector<'a>>) {
    let mut injector = req.into();
    global::get_text_map_propagator(|propagator| {
        let context = tracing::Span::current().context();
        propagator.inject_context(&context, &mut injector);
    });
}

/// Extracts the OTEL trace context from the provided request and sets it as the parent of the
/// current span.
pub fn extract_trace_context<'a>(req: impl Into<HeaderExtractor<'a>>) {
    let extractor = req.into();
    let parent_context =
        global::get_text_map_propagator(|propagator| propagator.extract(&extractor));
    tracing::Span::current().set_parent(parent_context);
}

pub enum HeaderInjector<'a> {
    Http0(&'a mut http0::HeaderMap),
    Http1(&'a mut http1::HeaderMap),
}

impl<'a> Injector for HeaderInjector<'a> {
    fn set(&mut self, key: &str, value: String) {
        match self {
            HeaderInjector::Http0(headers) => {
                if let Ok(name) = http0::header::HeaderName::from_bytes(key.as_bytes()) {
                    if let Ok(val) = http0::header::HeaderValue::from_str(&value) {
                        headers.insert(name, val);
                    }
                }
            }
            HeaderInjector::Http1(headers) => {
                if let Ok(name) = http1::header::HeaderName::from_bytes(key.as_bytes()) {
                    if let Ok(val) = http1::header::HeaderValue::from_str(&value) {
                        headers.insert(name, val);
                    }
                }
            }
        }
    }
}

impl<'a, T> From<&'a mut http0::Request<T>> for HeaderInjector<'a> {
    fn from(req: &'a mut http0::Request<T>) -> Self {
        Self::Http0(req.headers_mut())
    }
}

impl<'a, T> From<&'a mut http1::Request<T>> for HeaderInjector<'a> {
    fn from(req: &'a mut http1::Request<T>) -> Self {
        Self::Http1(req.headers_mut())
    }
}

pub enum HeaderExtractor<'a> {
    Http0(&'a http0::HeaderMap),
    Http1(&'a http1::HeaderMap),
}

impl<'a> Extractor for HeaderExtractor<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        match self {
            HeaderExtractor::Http0(headers) => {
                headers.get(key).map(|v| v.to_str().unwrap_or_default())
            }
            HeaderExtractor::Http1(headers) => {
                headers.get(key).map(|v| v.to_str().unwrap_or_default())
            }
        }
    }

    fn keys(&self) -> Vec<&str> {
        match self {
            HeaderExtractor::Http0(headers) => headers.keys().map(|k| k.as_str()).collect(),
            HeaderExtractor::Http1(headers) => headers.keys().map(|k| k.as_str()).collect(),
        }
    }
}

impl<'a, T> From<&'a http0::Request<T>> for HeaderExtractor<'a> {
    fn from(req: &'a http0::Request<T>) -> Self {
        Self::Http0(req.headers())
    }
}

impl<'a, T> From<&'a http1::Request<T>> for HeaderExtractor<'a> {
    fn from(req: &'a http1::Request<T>) -> Self {
        Self::Http1(req.headers())
    }
}
