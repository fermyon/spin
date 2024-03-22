use std::path::PathBuf;

use anyhow::Context as _;
use spin_factor_key_value::KeyValueFactor;
use spin_factor_llm::LlmFactor;
use spin_factor_observe::ObserveFactor;
use spin_factor_outbound_http::OutboundHttpFactor;
use spin_factor_outbound_mqtt::{NetworkedMqttClient, OutboundMqttFactor};
use spin_factor_outbound_mysql::OutboundMysqlFactor;
use spin_factor_outbound_networking::OutboundNetworkingFactor;
use spin_factor_outbound_pg::OutboundPgFactor;
use spin_factor_outbound_redis::OutboundRedisFactor;
use spin_factor_sqlite::SqliteFactor;
use spin_factor_variables::VariablesFactor;
use spin_factor_wasi::{spin::SpinFilesMounter, WasiFactor};
use spin_factors::RuntimeFactors;
use spin_runtime_config::TomlRuntimeConfigSource;

#[derive(RuntimeFactors)]
pub struct TriggerFactors {
    pub wasi: WasiFactor,
    pub variables: VariablesFactor,
    pub key_value: KeyValueFactor,
    pub outbound_networking: OutboundNetworkingFactor,
    pub outbound_http: OutboundHttpFactor,
    pub sqlite: SqliteFactor,
    pub redis: OutboundRedisFactor,
    pub mqtt: OutboundMqttFactor,
    pub pg: OutboundPgFactor,
    pub mysql: OutboundMysqlFactor,
    pub llm: LlmFactor,
    pub observe: ObserveFactor,
}

impl TriggerFactors {
    pub fn new(
        state_dir: Option<PathBuf>,
        working_dir: impl Into<PathBuf>,
        allow_transient_writes: bool,
        default_key_value_label_resolver: impl spin_factor_key_value::DefaultLabelResolver + 'static,
        default_sqlite_label_resolver: impl spin_factor_sqlite::DefaultLabelResolver + 'static,
        use_gpu: bool,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            wasi: wasi_factor(working_dir, allow_transient_writes),
            variables: VariablesFactor::default(),
            key_value: KeyValueFactor::new(default_key_value_label_resolver),
            outbound_networking: outbound_networking_factor(),
            outbound_http: OutboundHttpFactor::new(),
            sqlite: SqliteFactor::new(default_sqlite_label_resolver),
            redis: OutboundRedisFactor::new(),
            mqtt: OutboundMqttFactor::new(NetworkedMqttClient::creator()),
            pg: OutboundPgFactor::new(),
            mysql: OutboundMysqlFactor::new(),
            llm: LlmFactor::new(
                spin_factor_llm::spin::default_engine_creator(state_dir, use_gpu)
                    .context("failed to configure LLM factor")?,
            ),
            observe: ObserveFactor::new(),
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

impl TryFrom<TomlRuntimeConfigSource<'_, '_>> for TriggerFactorsRuntimeConfig {
    type Error = anyhow::Error;

    fn try_from(value: TomlRuntimeConfigSource<'_, '_>) -> Result<Self, Self::Error> {
        Self::from_source(value)
    }
}
