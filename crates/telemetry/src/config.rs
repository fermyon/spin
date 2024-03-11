use std::{env, time::Duration};

use anyhow::Result;
use opentelemetry_otlp::{ExportConfig, Protocol};
use url::Url;

/// Provides configuration for the telemetry system.
#[derive(Default, Debug)]
pub struct Config {
    pub is_enabled: bool,
    pub trace_log: String,
    pub tracing_config: ExportConfig,
    pub _metrics_config: ExportConfig,
}

impl Config {
    /// Derive the configuration from environment variables.
    ///
    /// Telemetry will only be enabled if at least one relevant environment variable is set.
    pub fn from_env() -> Result<Self> {
        let mut config = Self::default();
        load_endpoint_from_env(&mut config)?;
        load_protocol_from_env(&mut config)?;
        load_timeout_from_env(&mut config)?;
        Ok(config)
    }
}

fn load_endpoint_from_env(config: &mut Config) -> Result<()> {
    if let Ok(endpoint_default) = env::var("OTEL_EXPORTER_OTLP_ENDPOINT") {
        config.is_enabled = true;
        let url = Url::parse(&endpoint_default)?;
        config.tracing_config.endpoint = url.join("/v1/traces")?.to_string();
        config._metrics_config.endpoint = url.join("/v1/metrics")?.to_string();
    }

    if let Ok(traces_endpoint) = env::var("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT") {
        config.is_enabled = true;
        config.tracing_config.endpoint = traces_endpoint;
    }

    if let Ok(metrics_endpoint) = env::var("OTEL_EXPORTER_OTLP_METRICS_ENDPOINT") {
        config.is_enabled = true;
        config._metrics_config.endpoint = metrics_endpoint;
    }

    Ok(())
}

fn load_protocol_from_env(config: &mut Config) -> Result<()> {
    if let Ok(protocol_default) = env::var("OTEL_EXPORTER_OTLP_PROTOCOL") {
        config.is_enabled = true;
        let protocol = str_to_protocol(&protocol_default)?;
        config.tracing_config.protocol = protocol;
        config._metrics_config.protocol = protocol;
    }

    if let Ok(traces_protocol) = env::var("OTEL_EXPORTER_OTLP_TRACES_PROTOCOL") {
        config.is_enabled = true;
        config.tracing_config.protocol = str_to_protocol(&traces_protocol)?;
    }

    if let Ok(metrics_protocol) = env::var("OTEL_EXPORTER_OTLP_METRICS_PROTOCOL") {
        config.is_enabled = true;
        config._metrics_config.protocol = str_to_protocol(&metrics_protocol)?;
    }

    Ok(())
}

fn load_timeout_from_env(config: &mut Config) -> Result<()> {
    if let Ok(timeout_default) = env::var("OTEL_EXPORTER_OTLP_TIMEOUT") {
        config.is_enabled = true;
        let timeout = Duration::from_millis(timeout_default.parse()?);
        config.tracing_config.timeout = timeout;
        config._metrics_config.timeout = timeout;
    }

    if let Ok(traces_timeout) = env::var("OTEL_EXPORTER_OTLP_TRACES_TIMEOUT") {
        config.is_enabled = true;
        config.tracing_config.timeout = Duration::from_millis(traces_timeout.parse()?);
    }

    if let Ok(metrics_timeout) = env::var("OTEL_EXPORTER_OTLP_METRICS_TIMEOUT") {
        config.is_enabled = true;
        config._metrics_config.timeout = Duration::from_millis(metrics_timeout.parse()?);
    }

    Ok(())
}

fn str_to_protocol(s: &str) -> Result<Protocol> {
    match s {
        "grpc" => Ok(Protocol::Grpc),
        "http/protobuf" => Ok(Protocol::HttpBinary),
        _ => Err(anyhow::anyhow!("Invalid protocol")),
    }
}
