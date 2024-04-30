use std::time::Duration;

use anyhow::{bail, Result};
use opentelemetry_otlp::MetricsExporterBuilder;
use opentelemetry_sdk::{
    metrics::{
        reader::{DefaultAggregationSelector, DefaultTemporalitySelector},
        PeriodicReader, SdkMeterProvider,
    },
    resource::{EnvResourceDetector, TelemetryResourceDetector},
    runtime, Resource,
};
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};
use tracing_subscriber::{filter::Filtered, layer::Layered, EnvFilter, Registry};

use crate::{detector::SpinResourceDetector, env::OtlpProtocol};

/// Constructs a layer for the tracing subscriber that sends metrics to an OTEL collector.
///
/// It pulls OTEL configuration from the environment based on the variables defined
/// [here](https://opentelemetry.io/docs/specs/otel/protocol/exporter/) and
/// [here](https://opentelemetry.io/docs/specs/otel/configuration/sdk-environment-variables/#general-sdk-configuration).
pub(crate) fn otel_metrics_layer(spin_version: String) -> Result<CustomMetricsLayer> {
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
    let exporter_builder: MetricsExporterBuilder = match OtlpProtocol::metrics_protocol_from_env() {
        OtlpProtocol::Grpc => opentelemetry_otlp::new_exporter().tonic().into(),
        OtlpProtocol::HttpProtobuf => opentelemetry_otlp::new_exporter().http().into(),
        OtlpProtocol::HttpJson => bail!("http/json OTLP protocol is not supported"),
    };
    let exporter = exporter_builder.build_metrics_exporter(
        Box::new(DefaultTemporalitySelector::new()),
        Box::new(DefaultAggregationSelector::new()),
    )?;

    let reader = PeriodicReader::builder(exporter, runtime::Tokio).build();
    let meter_provider = SdkMeterProvider::builder()
        .with_reader(reader)
        .with_resource(resource)
        .build();

    Ok(MetricsLayer::new(meter_provider))
}

#[macro_export]
/// Records an increment to the named counter with the given attributes.
///
/// The increment may only be an i64 or f64. You must not mix types for the same metric.
///
/// ```no_run
/// # use spin_telemetry::metrics::counter;
/// counter!(spin.metric_name = 1, metric_attribute = "value");
/// ```
macro_rules! counter {
    ($metric:ident $(. $suffixes:ident)*  = $metric_value:expr $(, $attrs:ident=$values:expr)*) => {
        tracing::trace!(counter.$metric $(. $suffixes)* = $metric_value $(, $attrs=$values)*);
    }
}

#[macro_export]
/// Adds an additional value to the distribution of the named histogram with the given attributes.
///
/// The increment may only be an i64 or f64. You must not mix types for the same metric.
///
/// ```no_run
/// # use spin_telemetry::metrics::histogram;
/// histogram!(spin.metric_name = 1.5, metric_attribute = "value");
/// ```
macro_rules! histogram {
    ($metric:ident $(. $suffixes:ident)*  = $metric_value:expr $(, $attrs:ident=$values:expr)*) => {
        tracing::trace!(histogram.$metric $(. $suffixes)* = $metric_value $(, $attrs=$values)*);
    }
}

#[macro_export]
/// Records an increment to the named monotonic counter with the given attributes.
///
/// The increment may only be a positive i64 or f64. You must not mix types for the same metric.
///
/// ```no_run
/// # use spin_telemetry::metrics::monotonic_counter;
/// monotonic_counter!(spin.metric_name = 1, metric_attribute = "value");
/// ```
macro_rules! monotonic_counter {
    ($metric:ident $(. $suffixes:ident)*  = $metric_value:expr $(, $attrs:ident=$values:expr)*) => {
        tracing::trace!(monotonic_counter.$metric $(. $suffixes)* = $metric_value $(, $attrs=$values)*);
    }
}

pub use counter;
pub use histogram;
pub use monotonic_counter;

/// This really large type alias is require to make the registry.with() pattern happy.
type CustomMetricsLayer = MetricsLayer<
    Layered<
        Option<
            Filtered<
                OpenTelemetryLayer<Registry, opentelemetry_sdk::trace::Tracer>,
                EnvFilter,
                Registry,
            >,
        >,
        Registry,
    >,
>;
