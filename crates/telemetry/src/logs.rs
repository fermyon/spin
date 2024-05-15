use std::{sync::OnceLock, time::Duration};

use anyhow::bail;
use opentelemetry::global;
use opentelemetry_otlp::LogExporterBuilder;
use opentelemetry_sdk::{
    resource::{EnvResourceDetector, TelemetryResourceDetector},
    Resource,
};
use tracing::Subscriber;
use tracing_subscriber::{registry::LookupSpan, EnvFilter, Layer};

use crate::{
    detector::SpinResourceDetector,
    env::{self, OtlpProtocol},
};

/// Takes a Spin application log and emits it as a tracing event. This acts as a compatibility layer
/// to easily get Spin app logs as events in our OTel traces.
///
/// Note that this compatibility layer is also how logs make it to the OTel collector. We use a
/// [tracing] layer that bridges these app log events into OTel logs.
pub fn app_log_to_tracing_event(buf: &[u8]) {
    static CELL: OnceLock<bool> = OnceLock::new();
    if *CELL.get_or_init(env::spin_disable_log_to_tracing) {
        return;
    }

    if let Ok(s) = std::str::from_utf8(buf) {
        tracing::info!(app_log = s);
    } else {
        tracing::info!(app_log = "Application log: <non-utf8 data>");
    }
}

/// Constructs a layer for the tracing subscriber that sends Spin app logs to an OTEL collector.
///
/// It pulls OTEL configuration from the environment based on the variables defined
/// [here](https://opentelemetry.io/docs/specs/otel/protocol/exporter/) and
/// [here](https://opentelemetry.io/docs/specs/otel/configuration/sdk-environment-variables/#general-sdk-configuration).
pub(crate) fn otel_logging_layer<S: Subscriber + for<'span> LookupSpan<'span>>(
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
    // combination of OTEL_EXPORTER_OTLP_PROTOCOL and OTEL_EXPORTER_OTLP_LOGS_PROTOCOL to
    // determine whether we should use http/protobuf or grpc.
    let exporter_builder: LogExporterBuilder = match OtlpProtocol::logs_protocol_from_env() {
        OtlpProtocol::Grpc => opentelemetry_otlp::new_exporter().tonic().into(),
        OtlpProtocol::HttpProtobuf => opentelemetry_otlp::new_exporter().http().into(),
        OtlpProtocol::HttpJson => bail!("http/json OTLP protocol is not supported"),
    };

    let provider = opentelemetry_sdk::logs::LoggerProvider::builder()
        .with_config(opentelemetry_sdk::logs::config().with_resource(resource))
        .with_batch_exporter(
            exporter_builder.build_log_exporter()?,
            opentelemetry_sdk::runtime::Tokio,
        )
        .build();

    // We only want to pass through our Spin app logs to OTel
    let filter = EnvFilter::new("off,[{app_log}]=trace");

    let layer = opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge::new(&provider)
        .with_filter(filter);

    global::set_logger_provider(provider);

    Ok(layer)
}
