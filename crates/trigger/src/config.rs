use std::path::PathBuf;

use anyhow::Result;
use serde::Deserialize;
use toml;

// Config for config providers and wasmtime config
#[derive(Debug, Default, Deserialize)]
pub struct TriggerExecutorBuilderConfig {
    #[serde(rename = "config_provider", default)]
    pub config_providers: Vec<ConfigProvider>,

    #[serde(rename = "wasmtime", default)]
    pub wasmtime_config: WasmtimeConfig,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ConfigProvider {
    Vault(VaultConfig),
}

// Vault config to initialize vault provider
#[derive(Debug, Default, Deserialize)]
pub struct VaultConfig {
    pub url: String,
    pub token: String,
    pub mount: String,
    pub prefix: Option<String>,
}

// Wasmtime config to initialize wasmtime engine
#[derive(Debug, Default, Deserialize)]
pub struct WasmtimeConfig {
    pub cache_file: Option<String>,

    #[serde(default = "default_wask_backtrace_details")]
    pub wasm_backtrace_details: String,
}

impl TriggerExecutorBuilderConfig {
    pub fn load_from_file(config_file: Option<PathBuf>) -> Result<Self> {
        let config_file = match config_file {
            Some(p) => p,
            None => {
                return Ok(Self::default());
            }
        };
        let content = std::fs::read_to_string(config_file)?;
        let config: TriggerExecutorBuilderConfig = toml::from_str(&content)?;
        Ok(config)
    }
}

fn default_wask_backtrace_details() -> String {
    "Disable".to_string()
}
