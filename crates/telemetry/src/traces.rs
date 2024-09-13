use std::time::Duration;

use anyhow::bail;
use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::SpanExporterBuilder;
use opentelemetry_sdk::{
    resource::{EnvResourceDetector, TelemetryResourceDetector},
    Resource,
};
use tracing::Subscriber;
use tracing_subscriber::{registry::LookupSpan, EnvFilter, Layer};

use crate::detector::SpinResourceDetector;
use crate::env::OtlpProtocol;

/// Constructs a layer for the tracing subscriber that sends spans to an OTEL collector.
///
/// It pulls OTEL configuration from the environment based on the variables defined
/// [here](https://opentelemetry.io/docs/specs/otel/protocol/exporter/) and
/// [here](https://opentelemetry.io/docs/specs/otel/configuration/sdk-environment-variables/#general-sdk-configuration).
pub(crate) fn otel_tracing_layer<S: Subscriber + for<'span> LookupSpan<'span>>(
    spin_version: String,
) -> anyhow::Result<impl Layer<S>> {
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
    let exporter: SpanExporterBuilder = match OtlpProtocol::traces_protocol_from_env() {
        OtlpProtocol::Grpc => opentelemetry_otlp::new_exporter().tonic().into(),
        OtlpProtocol::HttpProtobuf => opentelemetry_otlp::new_exporter().http().into(),
        OtlpProtocol::HttpJson => bail!("http/json OTLP protocol is not supported"),
    };

    let tracer_provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(opentelemetry_sdk::trace::Config::default().with_resource(resource))
        .install_batch(opentelemetry_sdk::runtime::Tokio)?;

    let env_filter = match EnvFilter::try_from_env("SPIN_OTEL_TRACING_LEVEL") {
        Ok(filter) => filter,
        // If it isn't set or it fails to parse default to info
        Err(_) => EnvFilter::new("info"),
    };

    Ok(tracing_opentelemetry::layer()
        .with_tracer(tracer_provider.tracer("spin"))
        .with_threads(false)
        .with_filter(env_filter))
}
