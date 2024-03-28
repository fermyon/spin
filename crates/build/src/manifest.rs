use anyhow::Result;
use serde::Deserialize;
use std::{collections::BTreeMap, path::Path};

use spin_manifest::{schema::v2, ManifestVersion};

/// Returns a map of component IDs to [`v2::ComponentBuildConfig`]s for the
/// given (v1 or v2) manifest path.
pub async fn component_build_configs(
    manifest_file: impl AsRef<Path>,
) -> Result<Vec<ComponentBuildInfo>> {
    let manifest_text = tokio::fs::read_to_string(manifest_file).await?;
    Ok(match ManifestVersion::detect(&manifest_text)? {
        ManifestVersion::V1 => {
            let v1: ManifestV1BuildInfo = toml::from_str(&manifest_text)?;
            v1.components
        }
        ManifestVersion::V2 => {
            let v2: ManifestV2BuildInfo = toml::from_str(&manifest_text)?;
            let inlines = v2.triggers.values().flat_map(|triggers| {
                triggers
                    .iter()
                    .flat_map(|tr| tr.component_specs())
                    .filter_map(|spec| spec.buildinfo())
            });
            v2.components
                .into_iter()
                .map(|(id, mut c)| {
                    c.id = id;
                    c
                })
                .chain(inlines)
                .collect()
        }
    })
}

#[derive(Deserialize)]
pub struct TriggerBuildInfo {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub id: String,
    /// `component = ...`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub component: Option<ComponentSpec>,
    /// `components = { ... }`
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub components: indexmap::IndexMap<String, OneOrManyComponentSpecs>,
}

impl TriggerBuildInfo {
    fn component_specs(&self) -> Vec<&ComponentSpec> {
        match &self.component {
            Some(spec) => vec![spec],
            None => self
                .components
                .values()
                .flat_map(|specs| &specs.0)
                .collect(),
        }
    }
}

/// One or many `ComponentSpec`(s)
#[derive(Deserialize)]
#[serde(transparent)]
pub struct OneOrManyComponentSpecs(#[serde(with = "one_or_many")] pub Vec<ComponentSpec>);

/// Component reference or inline definition
#[derive(Deserialize)]
#[serde(untagged, try_from = "toml::Value")]
pub enum ComponentSpec {
    /// `"component-id"`
    Reference(String),
    /// `{ ... }`
    Inline(Box<ComponentBuildInfo>),
}

impl ComponentSpec {
    fn buildinfo(&self) -> Option<ComponentBuildInfo> {
        match self {
            Self::Reference(_) => None, // Will be picked up from `components` section
            Self::Inline(cbi) => Some(*cbi.clone()),
        }
    }
}

impl TryFrom<toml::Value> for ComponentSpec {
    type Error = toml::de::Error;

    fn try_from(value: toml::Value) -> Result<Self, Self::Error> {
        match value.as_str() {
            Some(s) => Ok(ComponentSpec::Reference(s.to_string())),
            None => Ok(ComponentSpec::Inline(Box::new(
                ComponentBuildInfo::deserialize(value)?,
            ))),
        }
    }
}

#[derive(Clone, Deserialize)]
pub struct ComponentBuildInfo {
    #[serde(default)]
    pub id: String,
    pub build: Option<v2::ComponentBuildConfig>,
}

#[derive(Deserialize)]
struct ManifestV1BuildInfo {
    #[serde(rename = "component")]
    components: Vec<ComponentBuildInfo>,
}

#[derive(Deserialize)]
struct ManifestV2BuildInfo {
    #[serde(rename = "trigger")]
    pub triggers: indexmap::IndexMap<String, Vec<TriggerBuildInfo>>,
    #[serde(default, rename = "component")]
    components: BTreeMap<String, ComponentBuildInfo>,
}

mod one_or_many {
    use serde::{Deserialize, Deserializer};

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
