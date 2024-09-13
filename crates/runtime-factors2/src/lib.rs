mod build;

pub use build::FactorsBuilder;
use spin_factor_llm2::LlmFactor;

use std::path::PathBuf;

use spin_common::arg_parser::parse_kv;
use spin_factor_key_value::KeyValueFactor;
use spin_factor_outbound_http::OutboundHttpFactor;
use spin_factor_outbound_networking::OutboundNetworkingFactor;
use spin_factor_variables::VariablesFactor;
use spin_factor_wasi::{spin::SpinFilesMounter, WasiFactor};
use spin_factors::{FactorRuntimeConfigSource, RuntimeFactors};
use spin_runtime_config::{ResolvedRuntimeConfig, TomlRuntimeConfigSource};

#[derive(RuntimeFactors)]
pub struct TriggerFactors {
    pub wasi: WasiFactor,
    pub variables: VariablesFactor,
    pub key_value: KeyValueFactor,
    pub outbound_networking: OutboundNetworkingFactor,
    pub outbound_http: OutboundHttpFactor,
    pub llm: LlmFactor,
}

impl TriggerFactors {
    pub fn new(
        working_dir: impl Into<PathBuf>,
        allow_transient_writes: bool,
        default_key_value_label_resolver: impl spin_factor_key_value::DefaultLabelResolver + 'static,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            wasi: wasi_factor(working_dir, allow_transient_writes),
            variables: VariablesFactor::default(),
            key_value: KeyValueFactor::new(default_key_value_label_resolver),
            outbound_networking: outbound_networking_factor(),
            outbound_http: OutboundHttpFactor::default(),
            llm: LlmFactor::new(),
        })
    }
}

fn wasi_factor(working_dir: impl Into<PathBuf>, allow_transient_writes: bool) -> WasiFactor {
    WasiFactor::new(SpinFilesMounter::new(working_dir, allow_transient_writes))
}

fn outbound_networking_factor() -> OutboundNetworkingFactor {
    fn disallowed_host_handler(scheme: &str, authority: &str) {
        let host_pattern = format!("{scheme}://{authority}");
        tracing::error!("Outbound network destination not allowed: {host_pattern}");
        if scheme.starts_with("http") && authority == "self" {
            terminal::warn!("A component tried to make an HTTP request to its own app but it does not have permission.");
        } else {
            terminal::warn!(
                "A component tried to make an outbound network connection to disallowed destination '{host_pattern}'."
            );
        };
        eprintln!("To allow this request, add 'allowed_outbound_hosts = [\"{host_pattern}\"]' to the manifest component section.");
    }

    let mut factor = OutboundNetworkingFactor::new();
    factor.set_disallowed_host_handler(disallowed_host_handler);
    factor
}

/// Options for building a [`TriggerFactors`].
#[derive(Default, clap::Args)]
pub struct TriggerAppArgs {
    /// Set the static assets of the components in the temporary directory as writable.
    #[clap(long = "allow-transient-write")]
    pub allow_transient_write: bool,

    /// Set a key/value pair (key=value) in the application's
    /// default store. Any existing value will be overwritten.
    /// Can be used multiple times.
    #[clap(long = "key-value", parse(try_from_str = parse_kv))]
    pub key_values: Vec<(String, String)>,
}

impl From<ResolvedRuntimeConfig<TriggerFactorsRuntimeConfig>> for TriggerFactorsRuntimeConfig {
    fn from(value: ResolvedRuntimeConfig<TriggerFactorsRuntimeConfig>) -> Self {
        value.runtime_config
    }
}

impl TryFrom<TomlRuntimeConfigSource<'_, '_>> for TriggerFactorsRuntimeConfig {
    type Error = anyhow::Error;

    fn try_from(mut value: TomlRuntimeConfigSource<'_, '_>) -> Result<Self, Self::Error> {
        Ok(TriggerFactorsRuntimeConfig {
            wasi: <TomlRuntimeConfigSource<'_,'_> as FactorRuntimeConfigSource<WasiFactor>>::get_runtime_config(&mut value)?,
            variables: <TomlRuntimeConfigSource<'_,'_> as FactorRuntimeConfigSource<VariablesFactor>>::get_runtime_config(&mut value)?,
            key_value: <TomlRuntimeConfigSource<'_,'_> as FactorRuntimeConfigSource<KeyValueFactor>>::get_runtime_config(&mut value)?,
            outbound_networking: <TomlRuntimeConfigSource<'_,'_> as FactorRuntimeConfigSource<OutboundNetworkingFactor>>::get_runtime_config(&mut value)?,
            outbound_http: <TomlRuntimeConfigSource<'_,'_> as FactorRuntimeConfigSource<OutboundHttpFactor>>::get_runtime_config(&mut value)?,
            llm: None
        })
    }
}
