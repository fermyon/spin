use std::collections::{HashMap, HashSet};

use anyhow::Context;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

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
    pub tags: Option<HashSet<String>>,
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

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", untagged)]
pub(crate) enum RawInstalledFrom {
    Git { git: String },
    File { dir: String },
}

pub(crate) fn parse_installed_from(text: impl AsRef<str>) -> Option<RawInstalledFrom> {
    // If we can't load it then it's not worth an error
    toml::from_str(text.as_ref()).ok()
}
