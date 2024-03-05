use http::{HeaderMap, Request};
use opentelemetry::{
    global,
    propagation::Extractor,
    sdk::{
        propagation::TraceContextPropagator,
        trace::{self, Tracer},
        Resource,
    },
};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_semantic_conventions::resource::{SERVICE_NAME, SERVICE_VERSION};
use tracing::metadata::LevelFilter;
use tracing_opentelemetry::{OpenTelemetryLayer, OpenTelemetrySpanExt};
use tracing_subscriber::{Layer, Registry};

use super::ServiceDescription;

/// Associate the current span with the incoming requests trace context.
pub fn accept_trace<T>(req: &Request<T>) {
    tracing::info!("headers map {:?}", &HeaderExtractor(req.headers()).keys());
    let parent_context = global::get_text_map_propagator(|propagator| {
        propagator.extract(&HeaderExtractor(req.headers()))
    });
    tracing::info!("parent context {:?}", parent_context);
    tracing::info!(
        "current::span context {:?}",
        tracing::Span::current().context()
    );
    tracing::Span::current().set_parent(parent_context);
    tracing::info!(
        "current::span metadata {:?}",
        tracing::Span::current().metadata()
    );
}

pub(crate) fn otel_tracing_layer(
    service: ServiceDescription,
    endpoint: String,
) -> anyhow::Result<
    tracing_subscriber::filter::Filtered<
        OpenTelemetryLayer<Registry, Tracer>,
        LevelFilter,
        Registry,
    >,
> {
    opentelemetry::global::set_text_map_propagator(TraceContextPropagator::new());

    let service_metadata = vec![
        SERVICE_NAME.string(service.name),
        SERVICE_VERSION.string(service.version),
    ];

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

struct HeaderExtractor<'a>(&'a HeaderMap);

impl<'a> Extractor for HeaderExtractor<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|v| {
            let s = v.to_str();
            if let Err(ref error) = s {
                tracing::warn!(%error, ?v, "cannot convert header value to ASCII")
            };
            tracing::info!("header key: {}, value: {}", key, s.as_ref().unwrap());
            s.ok()
        })
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|k| k.as_str()).collect()
    }
}
