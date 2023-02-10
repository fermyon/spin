use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// TODO
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "spin_version")]
pub enum BuildAppInfoAnyVersion {
    /// TODO
    #[serde(rename = "1")]
    V1(BuildAppInfoV1),
}

/// TODO
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct BuildAppInfoV1 {
    /// TODO
    #[serde(rename = "component")]
    pub components: Vec<RawComponentManifest>,
}

/// TODO
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct RawComponentManifest {
    /// TODO
    pub id: String,
    /// TODO
    pub build: Option<RawBuildConfig>,
}

/// TODO
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct RawBuildConfig {
    /// TODO
    pub command: String,
    /// TODO
    pub workdir: Option<PathBuf>,
    /// TODO
    pub watch: Option<Vec<String>>,
}
