use opentelemetry::{
    global,
    propagation::{Extractor, Injector},
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_otlp::{OTEL_EXPORTER_OTLP_ENDPOINT, OTEL_EXPORTER_OTLP_TRACES_ENDPOINT};
use opentelemetry_sdk::{trace::Tracer, Resource};
use opentelemetry_semantic_conventions::resource::SERVICE_NAME;
use tracing::metadata::LevelFilter;
use tracing_opentelemetry::{OpenTelemetryLayer, OpenTelemetrySpanExt};
use tracing_subscriber::{Layer, Registry};

use crate::config::Config;

/// Constructs a layer for the tracing subscriber that sends spans to an OTLP compliant collector.
///
/// In addition to the settings provided by [Config] it also pulls OTEL configuration from the
/// environment based on the variables defined
/// [here](https://opentelemetry.io/docs/specs/otel/protocol/exporter/).
pub(crate) fn otlp_tracing_layer(
    config: Config,
) -> anyhow::Result<
    tracing_subscriber::filter::Filtered<
        OpenTelemetryLayer<Registry, Tracer>,
        LevelFilter,
        Registry,
    >,
> {
    let mut service_metadata = vec![KeyValue::new(SERVICE_NAME, config.otel_service_name)];
    service_metadata.extend(config.otel_resource_attributes.inner());

    // This will configure the exporter based on the OTEL_EXPORTER_* environment variables. We
    // currently default to using the HTTP exporter but in the future we could select off of the
    // combination of OTEL_EXPORTER_OTLP_PROTOCOL and OTEL_EXPORTER_OTLP_TRACES_PROTOCOL to
    // determine whether we should use http/protobuf or grpc.
    let mut exporter = opentelemetry_otlp::new_exporter().http();

    // This mitigation was taken from https://github.com/neondatabase/neon/blob/main/libs/tracing-utils/src/lib.rs
    //
    // opentelemetry-otlp v0.15.0 has a bug in how it uses the
    // OTEL_EXPORTER_OTLP_ENDPOINT env variable. According to the
    // OpenTelemetry spec at
    // <https://github.com/open-telemetry/opentelemetry-specification/blob/main/specification/protocol/exporter.md#endpoint-urls-for-otlphttp>,
    // the full exporter URL is formed by appending "/v1/traces" to the value
    // of OTEL_EXPORTER_OTLP_ENDPOINT. However, opentelemetry-otlp only does
    // that with the grpc-tonic exporter. Other exporters, like the HTTP
    // exporter, use the URL from OTEL_EXPORTER_OTLP_ENDPOINT as is, without
    // appending "/v1/traces".
    //
    // See https://github.com/open-telemetry/opentelemetry-rust/pull/950
    //
    // Work around that by checking OTEL_EXPORTER_OTLP_ENDPOINT, and setting
    // the endpoint url with the "/v1/traces" path ourselves. If the bug is
    // fixed in a later version, we can remove this code. But if we don't
    // remember to remove this, it won't do any harm either, as the crate will
    // just ignore the OTEL_EXPORTER_OTLP_ENDPOINT setting when the endpoint
    // is set directly with `with_endpoint`.
    if std::env::var(OTEL_EXPORTER_OTLP_TRACES_ENDPOINT).is_err() {
        if let Ok(mut endpoint) = std::env::var(OTEL_EXPORTER_OTLP_ENDPOINT) {
            if !endpoint.ends_with('/') {
                endpoint.push('/');
            }
            endpoint.push_str("v1/traces");
            exporter = exporter.with_endpoint(endpoint);
        }
    }

    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(
            opentelemetry_sdk::trace::config().with_resource(Resource::new(service_metadata)),
        )
        .install_batch(opentelemetry_sdk::runtime::Tokio)?;

    Ok(tracing_opentelemetry::layer()
        .with_tracer(tracer)
        .with_threads(false)
        .with_filter(LevelFilter::INFO))
}

/// Injects the current W3C TraceContext into the provided request.
pub fn inject_trace_context<'a>(req: impl Into<HeaderInjector<'a>>) {
    let mut injector = req.into();
    global::get_text_map_propagator(|propagator| {
        let context = tracing::Span::current().context();
        propagator.inject_context(&context, &mut injector);
    });
}

/// Extracts the W3C TraceContext from the provided request and sets it as the parent of the
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
