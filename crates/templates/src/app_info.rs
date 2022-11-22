// Information about the application manifest that is of
// interest to the template system.  spin_loader does too
// much processing to fit our needs here.

use std::path::{Path, PathBuf};

use anyhow::Context;

use crate::store::TemplateLayout;

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "spin_version")]
pub(crate) enum AppInfo {
    /// A manifest with API version 1.
    #[serde(rename = "1")]
    V1(AppInfoV1),
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct AppInfoV1 {
    trigger: TriggerInfo,
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct TriggerInfo {
    #[serde(rename = "type")]
    trigger_type: String,
}

impl AppInfo {
    pub(crate) fn from_layout(layout: &TemplateLayout) -> Option<anyhow::Result<AppInfo>> {
        Self::layout_manifest_path(layout)
            .map(|manifest_path| Self::from_existent_file(&manifest_path))
    }

    pub(crate) fn from_file(manifest_path: &Path) -> Option<anyhow::Result<AppInfo>> {
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

    fn from_existent_file(manifest_path: &Path) -> anyhow::Result<AppInfo> {
        let manifest_text =
            std::fs::read_to_string(manifest_path).context("Can't read manifest file")?;
        toml::from_str(&manifest_text).context("Can't parse manifest file")
    }

    pub(crate) fn trigger_type(&self) -> &str {
        match self {
            Self::V1(info) => &info.trigger.trigger_type,
        }
    }
}
