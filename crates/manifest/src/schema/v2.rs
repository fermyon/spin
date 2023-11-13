use serde::{Deserialize, Serialize};
use spin_serde::FixedVersion;
pub use spin_serde::{KebabId, SnakeId};

pub use super::common::{ComponentBuildConfig, ComponentSource, Variable, WasiFilesMount};

pub(crate) type Map<K, V> = indexmap::IndexMap<K, V>;

/// App manifest
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppManifest {
    /// `spin_manifest_version = 2`
    pub spin_manifest_version: FixedVersion<2>,
    /// `[application]`
    pub application: AppDetails,
    /// `[variables]`
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub variables: Map<SnakeId, Variable>,
    /// `[[trigger.<type>]]`
    #[serde(rename = "trigger")]
    pub triggers: Map<String, Vec<Trigger>>,
    /// `[component.<id>]`
    #[serde(rename = "component")]
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub components: Map<KebabId, Component>,
}

/// App details
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppDetails {
    /// `name = "my-app"`
    pub name: String,
    /// `version = "1.0.0"`
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub version: String,
    /// `description = "App description"`
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// `authors = ["author@example.com"]`
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authors: Vec<String>,
    /// `[application.triggers.<type>]`
    #[serde(rename = "trigger", default, skip_serializing_if = "Map::is_empty")]
    pub trigger_global_configs: Map<String, toml::Table>,
}

/// Trigger configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Trigger {
    /// `id = "trigger-id"`
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub id: String,
    /// `component = ...`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub component: Option<ComponentSpec>,
    /// `components = { ... }`
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub components: Map<String, OneOrManyComponentSpecs>,
    /// Opaque trigger-type-specific config
    #[serde(flatten)]
    pub config: toml::Table,
}

/// One or many `ComponentSpec`(s)
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OneOrManyComponentSpecs(#[serde(with = "one_or_many")] pub Vec<ComponentSpec>);

/// Component reference or inline definition
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, untagged, try_from = "toml::Value")]
pub enum ComponentSpec {
    /// `"component-id"`
    Reference(KebabId),
    /// `{ ... }`
    Inline(Box<Component>),
}

impl TryFrom<toml::Value> for ComponentSpec {
    type Error = toml::de::Error;

    fn try_from(value: toml::Value) -> Result<Self, Self::Error> {
        if value.is_str() {
            Ok(ComponentSpec::Reference(KebabId::deserialize(value)?))
        } else {
            Ok(ComponentSpec::Inline(Box::new(Component::deserialize(
                value,
            )?)))
        }
    }
}

/// Component definition
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Component {
    /// `source = ...`
    pub source: ComponentSource,
    /// `description = "Component description"`
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// `variables = { name = "{{ app_var }}"}`
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub variables: Map<SnakeId, String>,
    /// `environment = { VAR = "value" }`
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub environment: Map<String, String>,
    /// `files = [...]`
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<WasiFilesMount>,
    /// `exclude_files = ["secrets/*"]`
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude_files: Vec<String>,
    /// `allowed_http_hosts = ["example.com"]`
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) allowed_http_hosts: Vec<String>,
    /// `allowed_outbound_hosts = ["redis://myredishost.com:6379"]`
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) allowed_outbound_hosts: Vec<String>,
    /// `key_value_stores = ["default"]`
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub key_value_stores: Vec<SnakeId>,
    /// `sqlite_databases = ["default"]`
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sqlite_databases: Vec<SnakeId>,
    /// `ai_models = ["llama2-chat"]`
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ai_models: Vec<KebabId>,
    /// Build configuration
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build: Option<ComponentBuildConfig>,
}

impl Component {
    /// Combine `allowed_outbound_hosts` with the deprecated `allowed_http_hosts` into
    /// one array all normalized to the syntax of `allowed_outbound_hosts`.
    pub fn normalized_allowed_outbound_hosts(&self) -> anyhow::Result<Vec<String>> {
        let normalized =
            crate::compat::convert_allowed_http_to_allowed_hosts(&self.allowed_http_hosts, false)?;
        if !normalized.is_empty() {
            terminal::warn!(
                "Use of the deprecated field `allowed_http_hosts` - to fix, \
            replace `allowed_http_hosts` with `allowed_outbound_hosts = {normalized:?}`",
            )
        }

        Ok(self
            .allowed_outbound_hosts
            .iter()
            .cloned()
            .chain(normalized)
            .collect())
    }
}

mod one_or_many {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<T, S>(vec: &Vec<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Serialize,
        S: Serializer,
    {
        if vec.len() == 1 {
            vec[0].serialize(serializer)
        } else {
            vec.serialize(serializer)
        }
    }

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<Vec<T>, D::Error>
    where
        T: Deserialize<'de>,
        D: Deserializer<'de>,
    {
        let value = toml::Value::deserialize(deserializer)?;
        if let Ok(val) = T::deserialize(value.clone()) {
            Ok(vec![val])
        } else {
            Vec::deserialize(value).map_err(serde::de::Error::custom)
        }
    }
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
        let manifest = AppManifest::deserialize(toml! {
            spin_manifest_version = 2
            [application]
            name = "trigger-configs"
            [application.trigger.fake]
            global_option = true
            [[trigger.fake]]
            component = { source = "inline.wasm" }
            option = true
        })
        .unwrap();

        FakeGlobalTriggerConfig::deserialize(
            manifest.application.trigger_global_configs["fake"].clone(),
        )
        .unwrap();

        FakeTriggerConfig::deserialize(manifest.triggers["fake"][0].config.clone()).unwrap();
    }

    #[test]
    fn test_valid_snake_ids() {
        for valid in ["default", "mixed_CASE_words", "letters1_then2_numbers345"] {
            if let Err(err) = SnakeId::try_from(valid.to_string()) {
                panic!("{valid:?} should be value: {err:?}");
            }
        }
    }

    #[test]
    fn test_invalid_snake_ids() {
        for invalid in [
            "",
            "kebab-case",
            "_leading_underscore",
            "trailing_underscore_",
            "double__underscore",
            "1initial_number",
            "unicode_snowpeople☃☃☃",
            "mIxEd_case",
            "MiXeD_case",
        ] {
            if SnakeId::try_from(invalid.to_string()).is_ok() {
                panic!("{invalid:?} should not be a valid SnakeId");
            }
        }
    }
}
