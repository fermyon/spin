use anyhow::{Context, Result};
use serde::Deserialize;
use spin_common::ui::quoted_path;
use std::{collections::BTreeMap, path::Path};

use spin_manifest::{schema::v2, ManifestVersion};

/// Returns a map of component IDs to [`v2::ComponentBuildConfig`]s for the
/// given (v1 or v2) manifest path.
pub async fn component_build_configs(
    manifest_file: impl AsRef<Path>,
) -> Result<Vec<ComponentBuildInfo>> {
    let app_root = manifest_file.as_ref().parent().unwrap();
    let manifest_text = tokio::fs::read_to_string(manifest_file.as_ref()).await?;
    match ManifestVersion::detect(&manifest_text)? {
        ManifestVersion::V1 => {
            let v1: ManifestV1BuildInfo = toml::from_str(&manifest_text)?;
            Ok(v1.components)
        }
        ManifestVersion::V2 => {
            let v2: ManifestV2BuildInfo = toml::from_str(&manifest_text)?;
            let inlines = v2.triggers.values().flat_map(|triggers| {
                triggers
                    .iter()
                    .flat_map(|tr| tr.component_specs())
                    .filter_map(|spec| spec.buildinfo(app_root))
            });
            v2.components
                .into_iter()
                .map(|(id, mut c)| {
                    c.id = id;
                    Ok(c)
                })
                .chain(inlines)
                .collect()
        }
    }
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
    /// `"@my-component/spin-component.toml"`
    External(std::path::PathBuf),
}

impl ComponentSpec {
    fn buildinfo(&self, app_root: &Path) -> Option<anyhow::Result<ComponentBuildInfo>> {
        match self {
            Self::Reference(_) => None, // Will be picked up from `components` section
            Self::Inline(cbi) => Some(Ok(*cbi.clone())),
            Self::External(path) => Some(load_cbi_from(path, app_root)),
        }
    }
}

impl TryFrom<toml::Value> for ComponentSpec {
    type Error = toml::de::Error;

    fn try_from(value: toml::Value) -> Result<Self, Self::Error> {
        match value.as_str() {
            Some(s) => match s.strip_prefix('@') {
                Some(path) => Ok(ComponentSpec::External(std::path::PathBuf::from(path))),
                None => Ok(ComponentSpec::Reference(s.to_string())),
            }
            None => Ok(ComponentSpec::Inline(Box::new(
                ComponentBuildInfo::deserialize(value)?,
            ))),
        }
    }
}

fn load_cbi_from(path: &Path, app_root: &Path) -> anyhow::Result<ComponentBuildInfo> {
    // moar duplication, we hates it precious
    let abs_path = app_root.join(&path);
    let (abs_path, containing_dir) = if abs_path.is_file() {
        (abs_path, path.parent().unwrap().to_owned())
    } else if abs_path.is_dir() {
        let inferred = abs_path.join("spin-component.toml");
        if inferred.is_file() {
            (inferred, path.to_owned())
        } else {
            anyhow::bail!("{} does not contain a spin-component.toml file", quoted_path(&abs_path));
        }
    } else {
        anyhow::bail!("{} does not exist", quoted_path(abs_path));
    };

    let toml_text = std::fs::read_to_string(&abs_path)?;
    let mut component: ComponentBuildInfo = toml::from_str(&toml_text)
        .with_context(|| format!("{} is not a valid component manifest", quoted_path(&abs_path)))?;

    if let Some(build) = &mut component.build {
        let workdir = match &build.workdir {
            Some(w) => containing_dir.join(w).to_string_lossy().to_string(),
            None => containing_dir.to_string_lossy().to_string(),
        };
        build.workdir = Some(workdir);
    }

    Ok(component)
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
