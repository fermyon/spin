/// Returns a boolean indicating if the OTEL layer should be enabled.
///
/// It is considered enabled if any of the following environment variables are set and not empty:
/// - `OTEL_EXPORTER_OTLP_ENDPOINT`
/// - `OTEL_EXPORTER_OTLP_TRACES_ENDPOINT`
/// - `OTEL_EXPORTER_OTLP_METRICS_ENDPOINT`
///
/// Note that this is overridden if OTEL_SDK_DISABLED is set and not empty.
pub(crate) fn otel_enabled() -> bool {
    const ENABLING_VARS: &[&str] = &[
        "OTEL_EXPORTER_OTLP_ENDPOINT",
        "OTEL_EXPORTER_OTLP_TRACES_ENDPOINT",
        "OTEL_EXPORTER_OTLP_METRICS_ENDPOINT",
    ];
    ENABLING_VARS
        .iter()
        .any(|key| std::env::var_os(key).is_some_and(|val| !val.is_empty()))
}

/// Returns a boolean indicating if the OTEL SDK should be disabled for all signals.
///
/// It is considered disabled if the environment variable `OTEL_SDK_DISABLED` is set and not empty.
pub(crate) fn otel_sdk_disabled() -> bool {
    std::env::var_os("OTEL_SDK_DISABLED").is_some_and(|val| !val.is_empty())
}

/// The protocol to use for OTLP exporter.
pub(crate) enum OtlpProtocol {
    Grpc,
    HttpProtobuf,
    HttpJson,
}

impl OtlpProtocol {
    /// Returns the protocol to be used for exporting traces as defined by the environment.
    pub(crate) fn traces_protocol_from_env() -> Self {
        let trace_protocol = std::env::var("OTEL_EXPORTER_OTLP_TRACES_PROTOCOL");
        let general_protocol = std::env::var("OTEL_EXPORTER_OTLP_PROTOCOL");
        let protocol = trace_protocol.unwrap_or(general_protocol.unwrap_or_default());

        match protocol.as_str() {
            "grpc" => Self::Grpc,
            "http/protobuf" => Self::HttpProtobuf,
            "http/json" => Self::HttpJson,
            _ => Self::HttpProtobuf,
        }
    }
}
