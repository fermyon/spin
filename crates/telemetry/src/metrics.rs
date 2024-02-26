use opentelemetry::{
    runtime,
    sdk::{export::metrics::aggregation::delta_temporality_selector, metrics::selectors},
};
use opentelemetry_otlp::{ExportConfig, Protocol, WithExportConfig};
use std::time::Duration;

/// Initialize the OpenTelemetry metrics pipeline
pub(crate) fn init_otel(endpoint: String) -> anyhow::Result<()> {
    let export_config = ExportConfig {
        endpoint,
        timeout: Duration::from_secs(3),
        protocol: Protocol::Grpc,
    };

    opentelemetry_otlp::new_pipeline()
        .metrics(
            selectors::simple::inexpensive(),
            delta_temporality_selector(),
            runtime::Tokio,
        )
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_export_config(export_config),
        )
        .with_period(Duration::from_secs(3))
        .with_timeout(Duration::from_secs(10))
        .build()?;

    Ok(())
}
