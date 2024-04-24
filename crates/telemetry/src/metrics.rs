use std::{collections::HashMap, sync::RwLock, time::Duration};

use anyhow::bail;
use lazy_static::lazy_static;
use opentelemetry::{
    global,
    metrics::{Counter, Histogram},
    Key, KeyValue, Value,
};
use opentelemetry_otlp::MetricsExporterBuilder;
use opentelemetry_sdk::{
    resource::{EnvResourceDetector, TelemetryResourceDetector},
    runtime, Resource,
};

use crate::{detector::SpinResourceDetector, env::OtlpProtocol};

/// Initializes the OTel metrics pipeline.
///
/// It pulls OTEL configuration from the environment based on the variables defined
/// [here](https://opentelemetry.io/docs/specs/otel/protocol/exporter/) and
/// [here](https://opentelemetry.io/docs/specs/otel/configuration/sdk-environment-variables/#general-sdk-configuration).
pub(crate) fn init(spin_version: String) -> anyhow::Result<()> {
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
    let exporter: MetricsExporterBuilder = match OtlpProtocol::metrics_protocol_from_env() {
        OtlpProtocol::Grpc => opentelemetry_otlp::new_exporter().tonic().into(),
        OtlpProtocol::HttpProtobuf => opentelemetry_otlp::new_exporter().http().into(),
        OtlpProtocol::HttpJson => bail!("http/json OTLP protocol is not supported"),
    };

    // TODO: Make period and other things configurable
    opentelemetry_otlp::new_pipeline()
        .metrics(runtime::Tokio)
        .with_exporter(exporter)
        .with_resource(resource)
        .with_period(Duration::from_secs(3))
        .with_timeout(Duration::from_secs(10))
        .build()?;

    Ok(())
}

macro_rules! generate_instrument_function_impl {
    ($type:ty, $instrument_type:ty, $instrument_name:ident, $op_name:ident) => {
        paste::item! {
            lazy_static! {
                static ref [< $type:upper _ $instrument_name:upper _INSTRUMENTS >]: RwLock<HashMap<String, $instrument_type<$type>>> =
                    RwLock::new(HashMap::new());
            }

            pub fn [< $type _ $instrument_name _ $op_name >] (metric_name: &str, value: $type, attrs: &[(impl Into<Key>, impl Into<Value>)]) {
                // TODO: Attrs is broken rn, we can fix this later if we like this approach

                // If we've already created the instrument, just increment it
                if let Some(instrument) = [< $type:upper _ $instrument_name:upper _INSTRUMENTS >].read().unwrap().get(metric_name) {
                    instrument.$op_name(value, &attrs);
                    return;
                }
                // Otherwise, create the instrument and insert it into the store using a write lock
                let meter = global::meter("this-is-the-meter-name"); // TODO: Should we support setting the meter name?
                let instrument = meter.[< $type _ $instrument_name >](metric_name.to_owned()).init();
                instrument.$op_name(value, &attrs);
                [< $type:upper _ $instrument_name:upper _INSTRUMENTS >]
                    .write()
                    .unwrap()
                    .insert(metric_name.to_string(), instrument);
                }
        }
    };
}

generate_instrument_function_impl!(u64, Counter, counter, add);
generate_instrument_function_impl!(f64, Counter, counter, add);
generate_instrument_function_impl!(u64, Histogram, histogram, record);
generate_instrument_function_impl!(f64, Histogram, histogram, record);

// TODO: Build some kind of warning about mixing up protocols and ports
// TODO: Write integration test for metrics
// TODO: How can I decouple all this o11y work to make it easier to test?
// TODO: Should I just directly return the instrument struct to the user to use?
// TODO: What's the deal with these observable instruments?
