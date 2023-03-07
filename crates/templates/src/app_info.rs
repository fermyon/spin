// Information about the application manifest that is of
// interest to the template system.  spin_loader does too
// much processing to fit our needs here.

use anyhow::Context;
use spin_loader::local::config::FixedStringVersion;
use std::path::{Path, PathBuf};

use crate::store::TemplateLayout;

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
pub(crate) enum AppInfo {
    V1Old {
        #[allow(dead_code)]
        spin_version: FixedStringVersion<1>,
        #[serde(flatten)]
        manifest: AppInfoV1,
    },
    V1New {
        #[allow(dead_code)]
        spin_manifest_version: FixedStringVersion<1>,
        #[serde(flatten)]
        manifest: AppInfoV1,
    },
}

impl AppInfo {
    pub fn as_v1(&self) -> &AppInfoV1 {
        match self {
            AppInfo::V1New { manifest, .. } => manifest,
            AppInfo::V1Old { manifest, .. } => manifest,
        }
    }
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

    fn from_existent_file(manifest_path: &Path) -> anyhow::Result<AppInfo> {
        let manifest_text =
            std::fs::read_to_string(manifest_path).context("Can't read manifest file")?;
        toml::from_str(&manifest_text).context("Can't parse manifest file")
    }

    pub fn trigger_type(&self) -> &str {
        &self.as_v1().trigger.trigger_type
    }
}
