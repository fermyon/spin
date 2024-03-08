use url::Url;

/// TODO
pub struct Config {
    pub is_enabled: bool,
    pub otel_exporter_otlp_traces_endpoint: Option<Url>,
    pub otel_exporter_otlp_metrics_endpoint: Option<Url>,
    pub otel_exporter_otlp_traces_insecure: bool,
    pub otel_exporter_otlp_metrics_insecure: bool,
    pub otel_exporter_otlp_traces_compression: Option<String>, // TODO: Make these an enum
    pub otel_exporter_otlp_metrics_compression: Option<String>, // TODO: Make these an enum
    pub otel_exporter_otlp_traces_timeout: Option<std::time::Duration>,
    pub otel_exporter_otlp_metrics_timeout: Option<std::time::Duration>,
    pub otel_exporter_otlp_traces_protocol: Option<String>, // TODO: Make these an enum
    pub otel_exporter_otlp_metrics_protocol: Option<String>, // TODO: Make these an enum
}

impl Default for Config {
    fn default() -> Self {
        Self {
            is_enabled: true,
            otel_exporter_otlp_traces_endpoint: None,
            otel_exporter_otlp_metrics_endpoint: None,
            otel_exporter_otlp_traces_insecure: false,
            otel_exporter_otlp_metrics_insecure: false,
            otel_exporter_otlp_traces_compression: None,
            otel_exporter_otlp_metrics_compression: None,
            otel_exporter_otlp_traces_timeout: None,
            otel_exporter_otlp_metrics_timeout: None,
            otel_exporter_otlp_traces_protocol: None,
            otel_exporter_otlp_metrics_protocol: None,
        }
    }
}

impl Config {
    /// TODO
    pub fn from_env() -> Self {
        // TODO: Totally rework this implementation
        let mut config = Self::default();
        if let Ok(endpoint) = std::env::var("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT") {
            config.otel_exporter_otlp_traces_endpoint = Some(Url::parse(&endpoint).unwrap());
        }
        if let Ok(endpoint) = std::env::var("OTEL_EXPORTER_OTLP_METRICS_ENDPOINT") {
            config.otel_exporter_otlp_metrics_endpoint = Some(Url::parse(&endpoint).unwrap());
        }
        if let Ok(insecure) = std::env::var("OTEL_EXPORTER_OTLP_TRACES_INSECURE") {
            config.otel_exporter_otlp_traces_insecure = insecure.parse().unwrap();
        }
        if let Ok(insecure) = std::env::var("OTEL_EXPORTER_OTLP_METRICS_INSECURE") {
            config.otel_exporter_otlp_metrics_insecure = insecure.parse().unwrap();
        }
        if let Ok(compression) = std::env::var("OTEL_EXPORTER_OTLP_TRACES_COMPRESSION") {
            config.otel_exporter_otlp_traces_compression = Some(compression);
        }
        if let Ok(compression) = std::env::var("OTEL_EXPORTER_OTLP_METRICS_COMPRESSION") {
            config.otel_exporter_otlp_metrics_compression = Some(compression);
        }
        if let Ok(timeout) = std::env::var("OTEL_EXPORTER_OTLP_TRACES_TIMEOUT") {
            config.otel_exporter_otlp_traces_timeout =
                Some(std::time::Duration::from_secs(timeout.parse().unwrap()));
        }
        if let Ok(timeout) = std::env::var("OTEL_EXPORTER_OTLP_METRICS_TIMEOUT") {
            config.otel_exporter_otlp_metrics_timeout =
                Some(std::time::Duration::from_secs(timeout.parse().unwrap()));
        }
        if let Ok(protocol) = std::env::var("OTEL_EXPORTER_OTLP_TRACES_PROTOCOL") {
            config.otel_exporter_otlp_traces_protocol = Some(protocol);
        }
        if let Ok(protocol) = std::env::var("OTEL_EXPORTER_OTLP_METRICS_PROTOCOL") {
            config.otel_exporter_otlp_metrics_protocol = Some(protocol);
        }
        config
    }
}
