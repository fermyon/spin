// Information about the application manifest that is of
// interest to the template system.  spin_loader does too
// much processing to fit our needs here.

use std::path::{Path, PathBuf};

use anyhow::ensure;
use serde::Deserialize;
use spin_manifest::schema::v1;

use crate::store::TemplateLayout;

pub(crate) struct AppInfo {
    trigger_type: String,
}

impl AppInfo {
    pub fn from_layout(layout: &TemplateLayout) -> Option<anyhow::Result<AppInfo>> {
        Self::layout_manifest_path(layout)
            .map(|manifest_path| Self::from_existent_file(&manifest_path))
    }

    pub fn from_file(manifest_path: &Path) -> Option<anyhow::Result<AppInfo>> {
        if manifest_path.exists() {
            Some(Self::from_existent_file(manifest_path))
        } else {
            None
        }
    }

    fn layout_manifest_path(layout: &TemplateLayout) -> Option<PathBuf> {
        let manifest_path = layout.content_dir().join("spin.toml");
        if manifest_path.exists() {
            Some(manifest_path)
        } else {
            None
        }
    }

    fn from_existent_file(manifest_path: &Path) -> anyhow::Result<Self> {
        let manifest_str = std::fs::read_to_string(manifest_path)?;
        let trigger_type = match spin_manifest::ManifestVersion::detect(&manifest_str)? {
            spin_manifest::ManifestVersion::V1 => {
                toml::from_str::<ManifestV1TriggerProbe>(&manifest_str)?
                    .trigger
                    .trigger_type
            }
            spin_manifest::ManifestVersion::V2 => {
                let triggers = toml::from_str::<ManifestV2TriggerProbe>(&manifest_str)?.trigger;
                let type_count = triggers.len();
                ensure!(
                    type_count == 1,
                    "only 1 trigger type currently supported; got {type_count}"
                );
                triggers.into_iter().next().unwrap().0
            }
        };
        Ok(Self { trigger_type })
    }

    pub fn trigger_type(&self) -> &str {
        &self.trigger_type
    }
}

#[derive(Deserialize)]
struct ManifestV1TriggerProbe {
    // `trigger = { type = "<type>", ...}`
    trigger: v1::AppTriggerV1,
}

#[derive(Deserialize)]
struct ManifestV2TriggerProbe {
    // `[trigger.<type>]`
    trigger: toml::value::Table,
}
