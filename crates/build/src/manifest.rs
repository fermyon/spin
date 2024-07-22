use anyhow::Result;
use serde::Deserialize;
use std::{collections::BTreeMap, path::Path};

use spin_manifest::{schema::v2, ManifestVersion};

use crate::deployment::DeploymentTargets;

/// Returns a map of component IDs to [`v2::ComponentBuildConfig`]s for the
/// given (v1 or v2) manifest path. If the manifest cannot be loaded, the
/// function attempts fallback: if fallback succeeds, result is Ok but the load error
/// is also returned via the second part of the return value tuple.
pub async fn component_build_configs(
    manifest_file: impl AsRef<Path>,
) -> Result<(
    Vec<ComponentBuildInfo>,
    DeploymentTargets,
    Result<spin_manifest::schema::v2::AppManifest, spin_manifest::Error>,
)> {
    let manifest = spin_manifest::manifest_from_file(&manifest_file);
    match manifest {
        Ok(mut manifest) => {
            spin_manifest::normalize::normalize_manifest(&mut manifest);
            let bc = build_configs_from_manifest(&manifest);
            let dt = deployment_targets_from_manifest(&manifest);
            Ok((bc, dt, Ok(manifest)))
        }
        Err(e) => {
            let bc = fallback_load_build_configs(&manifest_file).await?;
            let dt = fallback_load_deployment_targets(&manifest_file).await?;
            Ok((bc, dt, Err(e)))
        }
    }
}

fn build_configs_from_manifest(
    manifest: &spin_manifest::schema::v2::AppManifest,
) -> Vec<ComponentBuildInfo> {
    manifest
        .components
        .iter()
        .map(|(id, c)| ComponentBuildInfo {
            id: id.to_string(),
            build: c.build.clone(),
        })
        .collect()
}

fn deployment_targets_from_manifest(
    manifest: &spin_manifest::schema::v2::AppManifest,
) -> DeploymentTargets {
    let target_environments = manifest.application.targets.clone();
    // let components = manifest
    //     .components
    //     .iter()
    //     .map(|(id, c)| (id.to_string(), c.source.clone()))
    //     .collect();
    DeploymentTargets::new(target_environments)
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

async fn fallback_load_deployment_targets(
    manifest_file: impl AsRef<Path>,
) -> Result<DeploymentTargets> {
    // fn try_parse_component_source(c: (&String, &toml::Value)) -> Option<(String, spin_manifest::schema::v2::ComponentSource)> {
    //     let (id, ctab) = c;
    //     let cs = ctab.as_table()
    //         .and_then(|c| c.get("source"))
    //         .and_then(|cs| spin_manifest::schema::v2::ComponentSource::deserialize(cs.clone()).ok());
    //     cs.map(|cs| (id.to_string(), cs))
    // }
    let manifest_text = tokio::fs::read_to_string(manifest_file).await?;
    Ok(match ManifestVersion::detect(&manifest_text)? {
        ManifestVersion::V1 => Default::default(),
        ManifestVersion::V2 => {
            let table: toml::value::Table = toml::from_str(&manifest_text)?;
            let target_environments = table
                .get("application")
                .and_then(|a| a.as_table())
                .and_then(|t| t.get("targets"))
                .and_then(|arr| arr.as_array())
                .map(|v| v.as_slice())
                .unwrap_or_default()
                .iter()
                .filter_map(|t| t.as_str())
                .map(|s| s.to_owned())
                .collect();
            // let components = table
            //     .get("component")
            //     .and_then(|cs| cs.as_table())
            //     .map(|table| table.iter().filter_map(try_parse_component_source).collect())
            //     .unwrap_or_default();
            DeploymentTargets::new(target_environments)
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
