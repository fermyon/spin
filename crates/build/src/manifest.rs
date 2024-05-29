use anyhow::Result;
use serde::Deserialize;
use std::{collections::BTreeMap, path::Path};

use spin_manifest::{schema::v2, ManifestVersion};

/// Returns a map of component IDs to [`v2::ComponentBuildConfig`]s for the
/// given (v1 or v2) manifest path. If the manifest cannot be loaded, the
/// function attempts fallback: if fallback succeeds, result is Ok but the load error
/// is also returned via the second part of the return value tuple.
pub async fn component_build_configs(
    manifest_file: impl AsRef<Path>,
) -> Result<(Vec<ComponentBuildInfo>, Option<spin_manifest::Error>)> {
    let manifest = spin_manifest::manifest_from_file(&manifest_file);
    match manifest {
        Ok(manifest) => Ok((build_configs_from_manifest(manifest), None)),
        Err(e) => fallback_load_build_configs(&manifest_file)
            .await
            .map(|bc| (bc, Some(e))),
    }
}

fn build_configs_from_manifest(
    mut manifest: spin_manifest::schema::v2::AppManifest,
) -> Vec<ComponentBuildInfo> {
    spin_manifest::normalize::normalize_manifest(&mut manifest);

    manifest
        .components
        .into_iter()
        .map(|(id, c)| ComponentBuildInfo {
            id: id.to_string(),
            build: c.build,
        })
        .collect()
}

async fn fallback_load_build_configs(
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
