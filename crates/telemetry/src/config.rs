use envconfig::Envconfig;
use opentelemetry::KeyValue;
use opentelemetry_semantic_conventions::resource::SERVICE_VERSION;

/// Provides configuration for the telemetry system.
///
/// Largely consists of OpenTelemetry variables defined
/// [here](https://opentelemetry.io/docs/specs/otel/configuration/sdk-environment-variables/#general-sdk-configuration)
/// but also has some Spin specific configuration.
#[derive(Envconfig, Debug)]
pub struct Config {
    /// Disable the OpenTelemetry for all signals (traces and metrics).
    #[envconfig(from = "OTEL_SDK_DISABLED", default = "false")]
    pub otel_sdk_disabled: bool,

    /// Sets the value of the `service.name` resource attribute.
    #[envconfig(from = "OTEL_SERVICE_NAME", default = "spin")]
    pub otel_service_name: String,

    /// Key-value pairs to be used as resource attributes.
    #[envconfig(from = "OTEL_RESOURCE_ATTRIBUTES", default = "")]
    pub otel_resource_attributes: KeyValues,

    /// Determines the verbosity of the OpenTelemetry tracing layer.
    ///
    /// This is a Spin specific value. This is what allows us to have different levels of verbosity
    /// in the fmt layer and the otel layer. The fmt layer respects `RUST_LOG` and the otel layer
    /// respects `OTEL_TRACING_LEVEL`.
    #[envconfig(from = "OTEL_TRACING_LEVEL", default = "info")]
    pub otel_tracing_level: String,
}

impl Config {
    /// Initializes a new [Config] from the environment.
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Config::init_from_env()?)
    }

    /// TODO: This is a hack until I can figure out how to set default value for
    /// `otel_resource_attributes` with FromStr.
    pub fn set_version(&mut self, version: String) {
        self.otel_resource_attributes
            .0
            .push(KeyValue::new(SERVICE_VERSION, version));
    }
}

/// A list of [KeyValue] pairs to be used as resource attributes.
#[derive(Debug)]
pub struct KeyValues(Vec<KeyValue>);

impl KeyValues {
    /// Returns a reference to the inner list of key-value pairs.
    pub fn inner(self) -> Vec<KeyValue> {
        self.0
    }
}

impl std::str::FromStr for KeyValues {
    type Err = anyhow::Error;

    fn from_str(_s: &str) -> Result<Self, Self::Err> {
        // TODO: Get a working implementation of this
        Ok(KeyValues(Vec::new()))
    }
}
