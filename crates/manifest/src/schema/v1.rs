use serde::Deserialize;

use spin_serde::FixedStringVersion;

pub use super::common::{
    ComponentBuildConfig as ComponentBuildConfigV1, ComponentSource as ComponentSourceV1,
    Variable as VariableV1, WasiFilesMount as WasiFilesMountV1,
};

type Map<K, V> = indexmap::IndexMap<K, V>;

/// App manifest
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppManifestV1 {
    /// `spin_manifest_version = "1"`
    #[serde(alias = "spin_version")]
    #[allow(dead_code)]
    spin_manifest_version: FixedStringVersion<1>,
    /// `name = "my-app"`
    pub name: String,
    /// `version = "1.0.0"`
    #[serde(default)]
    pub version: String,
    /// `description = "App description"`
    #[serde(default)]
    pub description: String,
    /// `authors = ["author@example.com"]`
    #[serde(default)]
    pub authors: Vec<String>,
    /// `trigger = { ... }`
    pub trigger: AppTriggerV1,
    /// `[variables]`
    #[serde(default)]
    pub variables: Map<String, VariableV1>,
    /// `[[component]]`
    #[serde(rename = "component")]
    #[serde(default)]
    pub components: Vec<ComponentV1>,
}

/// App trigger config
#[derive(Deserialize)]
pub struct AppTriggerV1 {
    /// `type = "trigger-type"`
    #[serde(rename = "type")]
    pub trigger_type: String,
    /// Trigger config
    #[serde(flatten)]
    pub config: toml::Table,
}

/// Component definition
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentV1 {
    /// `id = "component-id"
    pub id: String,
    /// `source = ...`
    pub source: ComponentSourceV1,
    /// `[component.trigger]`
    pub trigger: toml::Table,
    /// `description = "Component description"`
    #[serde(default)]
    pub description: String,
    /// `config = { name = "{{ app_var }}"}`
    #[serde(default)]
    pub config: Map<String, String>,
    /// `environment = { VAR = "value" }`
    #[serde(default)]
    pub environment: Map<String, String>,
    /// `files = [...]`
    #[serde(default)]
    pub files: Vec<WasiFilesMountV1>,
    /// `exclude_files = ["secrets/*"]`
    #[serde(default)]
    pub exclude_files: Vec<String>,
    /// `allowed_http_hosts = ["example.com"]`
    #[serde(default)]
    pub allowed_http_hosts: Vec<String>,
    /// `allowed_outbound_hosts` = ["redis://redis.com:6379"]`
    #[serde(default)]
    pub allowed_outbound_hosts: Option<Vec<String>>,
    /// `key_value_stores = ["default"]`
    #[serde(default)]
    pub key_value_stores: Vec<String>,
    /// `sqlite_databases = ["default"]`
    #[serde(default)]
    pub sqlite_databases: Vec<String>,
    /// `ai_models = ["llama2-chat"]`
    #[serde(default)]
    pub ai_models: Vec<String>,
    /// Build configuration
    #[serde(default)]
    pub build: Option<ComponentBuildConfigV1>,
}

#[cfg(test)]
mod tests {
    use toml::toml;

    use super::*;

    #[derive(Deserialize)]
    #[allow(dead_code)]
    struct FakeGlobalTriggerConfig {
        global_option: bool,
    }

    #[derive(Deserialize)]
    #[allow(dead_code)]
    struct FakeTriggerConfig {
        option: Option<bool>,
    }

    #[test]
    fn deserializing_trigger_configs() {
        let manifest = AppManifestV1::deserialize(toml! {
            spin_manifest_version = "1"
            name = "trigger-configs"
            trigger = { type = "fake", global_option = true }
            [[component]]
            id = "my-component"
            source = "example.wasm"
            [component.trigger]
            option = true
        })
        .unwrap();

        FakeGlobalTriggerConfig::deserialize(manifest.trigger.config).unwrap();

        FakeTriggerConfig::deserialize(manifest.components[0].trigger.clone()).unwrap();
    }
}
