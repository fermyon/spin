use std::time::Duration;

use anyhow::bail;
use opentelemetry_otlp::{SpanExporterBuilder, WithExportConfig};
use opentelemetry_otlp::{OTEL_EXPORTER_OTLP_ENDPOINT, OTEL_EXPORTER_OTLP_TRACES_ENDPOINT};
use opentelemetry_sdk::{
    resource::{EnvResourceDetector, TelemetryResourceDetector},
    trace::Tracer,
    Resource,
};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{EnvFilter, Layer, Registry};

use crate::detector::SpinResourceDetector;
use crate::env::OtlpProtocol;

/// Constructs a layer for the tracing subscriber that sends spans to an OTEL collector.
///
/// It pulls OTEL configuration from the environment based on the variables defined
/// [here](https://opentelemetry.io/docs/specs/otel/protocol/exporter/) and
/// [here](https://opentelemetry.io/docs/specs/otel/configuration/sdk-environment-variables/#general-sdk-configuration).
pub(crate) fn otel_tracing_layer(
    spin_version: String,
) -> anyhow::Result<
    tracing_subscriber::filter::Filtered<OpenTelemetryLayer<Registry, Tracer>, EnvFilter, Registry>,
> {
    let resource = Resource::from_detectors(
        Duration::from_secs(5),
        vec![
            // Set service.name from env OTEL_SERVICE_NAME > env OTEL_RESOURCE_ATTRIBUTES > spin
            // Set service.version from Spin metadata
            Box::new(SpinResourceDetector::new(spin_version)),
            // Sets fields from env OTEL_RESOURCE_ATTRIBUTES
            Box::new(EnvResourceDetector::new()),
            // Sets telemetry.sdk{name, language, version}
            Box::new(TelemetryResourceDetector),
        ],
    );

    // This will configure the exporter based on the OTEL_EXPORTER_* environment variables. We
    // currently default to using the HTTP exporter but in the future we could select off of the
    // combination of OTEL_EXPORTER_OTLP_PROTOCOL and OTEL_EXPORTER_OTLP_TRACES_PROTOCOL to
    // determine whether we should use http/protobuf or grpc.
    let mut exporter: SpanExporterBuilder = match OtlpProtocol::traces_protocol_from_env() {
        OtlpProtocol::Grpc => opentelemetry_otlp::new_exporter().tonic().into(),
        OtlpProtocol::HttpProtobuf => opentelemetry_otlp::new_exporter().http().into(),
        OtlpProtocol::HttpJson => bail!("http/json OTLP protocol is not supported"),
    };
    if let Some(endpoint) = fix_endpoint_bug() {
        match exporter {
            SpanExporterBuilder::Tonic(inner) => exporter = inner.with_endpoint(endpoint).into(),
            SpanExporterBuilder::Http(inner) => exporter = inner.with_endpoint(endpoint).into(),
            _ => {}
        }
    }

    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(opentelemetry_sdk::trace::config().with_resource(resource))
        .install_batch(opentelemetry_sdk::runtime::Tokio)?;

    let env_filter = match EnvFilter::try_from_env("SPIN_OTEL_TRACING_LEVEL") {
        Ok(filter) => filter,
        // If it isn't set or it fails to parse default to info
        Err(_) => EnvFilter::new("info"),
    };

    Ok(tracing_opentelemetry::layer()
        .with_tracer(tracer)
        .with_threads(false)
        .with_filter(env_filter))
}

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
fn fix_endpoint_bug() -> Option<String> {
    if std::env::var(OTEL_EXPORTER_OTLP_TRACES_ENDPOINT).is_err() {
        if let Ok(mut endpoint) = std::env::var(OTEL_EXPORTER_OTLP_ENDPOINT) {
            if !endpoint.ends_with('/') {
                endpoint.push('/');
            }
            endpoint.push_str("v1/traces");
            return Some(endpoint);
        }
    }
    None
}
