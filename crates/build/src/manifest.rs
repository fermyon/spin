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
            v2.components
                .into_iter()
                .map(|(id, mut c)| {
                    c.id = id;
                    c
                })
                .collect()
        }
    })
}

#[derive(Deserialize)]
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
    #[serde(rename = "component")]
    components: BTreeMap<String, ComponentBuildInfo>,
}
