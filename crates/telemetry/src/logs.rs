use std::{ascii::escape_default, sync::OnceLock, time::Duration};

use anyhow::bail;
use opentelemetry::{
    global,
    logs::{Logger, LoggerProvider},
};
use opentelemetry_otlp::LogExporterBuilder;
use opentelemetry_sdk::{
    logs::{BatchConfigBuilder, BatchLogProcessor},
    resource::{EnvResourceDetector, TelemetryResourceDetector},
    Resource,
};

use crate::{
    detector::SpinResourceDetector,
    env::{self, otel_logs_enabled, OtlpProtocol},
};

/// Handle an application log. Has the potential to both forward the log to OTel and to emit it as a
/// tracing event.
pub fn handle_app_log(buf: &[u8]) {
    app_log_to_otel(buf);
    app_log_to_tracing_event(buf);
}

/// Forward the app log to OTel.
fn app_log_to_otel(buf: &[u8]) {
    if !otel_logs_enabled() {
        return;
    }

    let logger = global::logger_provider().logger("spin");
    if let Ok(s) = std::str::from_utf8(buf) {
        logger.emit(
            opentelemetry::logs::LogRecord::builder()
                .with_body(s.to_owned())
                .build(),
        );
    } else {
        logger.emit(
            opentelemetry::logs::LogRecord::builder()
                .with_body(escape_non_utf8_buf(buf))
                .with_attribute("app_log_non_utf8", true)
                .build(),
        );
    }
}

/// Takes a Spin application log and emits it as a tracing event. This acts as a compatibility layer
/// to easily get Spin app logs as events in our OTel traces.
fn app_log_to_tracing_event(buf: &[u8]) {
    static CELL: OnceLock<bool> = OnceLock::new();
    if *CELL.get_or_init(env::spin_disable_log_to_tracing) {
        return;
    }

    if let Ok(s) = std::str::from_utf8(buf) {
        tracing::info!(app_log = s);
    } else {
        tracing::info!(app_log_non_utf8 = escape_non_utf8_buf(buf));
    }
}

fn escape_non_utf8_buf(buf: &[u8]) -> String {
    buf.iter()
        .take(50)
        .map(|&x| escape_default(x).to_string())
        .collect::<String>()
}

/// Initialize the OTel logging backend.
pub(crate) fn init_otel_logging_backend(spin_version: String) -> anyhow::Result<()> {
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
        .with_log_processor(
            BatchLogProcessor::builder(
                exporter_builder.build_log_exporter()?,
                opentelemetry_sdk::runtime::Tokio,
            )
            .with_batch_config(BatchConfigBuilder::default().build())
            .build(),
        )
        .build();

    global::set_logger_provider(provider);

    Ok(())
}
