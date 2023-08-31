use serde::{Deserialize, Serialize};
use spin_loader::local::config::FixedStringVersion;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct BuildAppInfoAnyVersion {
    #[allow(dead_code)]
    #[serde(alias = "spin_version")]
    spin_manifest_version: FixedStringVersion<1>,
    #[serde(flatten)]
    manifest: BuildAppInfoV1,
}
impl BuildAppInfoAnyVersion {
    pub fn into_v1(self) -> BuildAppInfoV1 {
        self.manifest
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct BuildAppInfoV1 {
    #[serde(rename = "component")]
    pub components: Vec<RawComponentManifest>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct RawComponentManifest {
    pub id: String,
    pub build: Option<RawBuildConfig>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub(crate) struct RawBuildConfig {
    pub command: String,
    pub workdir: Option<PathBuf>,
    pub watch: Option<Vec<String>>,
}
