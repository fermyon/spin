use std::collections::HashMap;

use anyhow::Context;
use indexmap::IndexMap;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(tag = "manifest_version")]
pub(crate) enum RawTemplateManifest {
    /// A manifest with API version 1.
    #[serde(rename = "1")]
    V1(RawTemplateManifestV1),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub(crate) struct RawTemplateManifestV1 {
    pub id: String,
    pub description: Option<String>,
    pub trigger_type: Option<String>,
    pub new_application: Option<RawTemplateVariant>,
    pub add_component: Option<RawTemplateVariant>,
    pub parameters: Option<IndexMap<String, RawParameter>>,
    pub custom_filters: Option<Vec<RawCustomFilter>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub(crate) struct RawTemplateVariant {
    pub supported: Option<bool>,
    pub skip_files: Option<Vec<String>>,
    pub skip_parameters: Option<Vec<String>>,
    pub snippets: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub(crate) struct RawParameter {
    #[serde(rename = "type")]
    pub data_type: String,
    pub prompt: String,
    #[serde(rename = "default")]
    pub default_value: Option<String>,
    pub pattern: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub(crate) struct RawCustomFilter {
    pub name: String,
    pub wasm: String,
}

pub(crate) fn parse_manifest_toml(text: impl AsRef<str>) -> anyhow::Result<RawTemplateManifest> {
    toml::from_str(text.as_ref()).context("Failed to parse template manifest TOML")
}
