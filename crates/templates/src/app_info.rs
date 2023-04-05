// Information about the application manifest that is of
// interest to the template system.  spin_loader does too
// much processing to fit our needs here.

use anyhow::Context;
use spin_loader::local::config::{is_missing_tag_error, VersionTagLoader};
use std::path::{Path, PathBuf};

use crate::store::TemplateLayout;

#[derive(Debug)]
pub(crate) enum AppInfo {
    V1(AppInfoV1),
}

impl AppInfo {
    pub fn as_v1(&self) -> &AppInfoV1 {
        match self {
            AppInfo::V1(manifest) => manifest,
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
        raw_manifest_from_str(&manifest_text).context("Can't parse manifest file")
    }

    pub fn trigger_type(&self) -> &str {
        &self.as_v1().trigger.trigger_type
    }
}

fn raw_manifest_from_str(buf: &str) -> anyhow::Result<AppInfo> {
    use serde::Deserialize;
    let tl = toml::from_str(buf);
    let tl = if is_missing_tag_error(&tl) {
        tl.context("Manifest must contain spin_manifest_version with a value of \"1\"")?
    } else {
        tl?
    };

    match tl {
        VersionTagLoader::OldV1 { rest, .. } => {
            let raw = AppInfoV1::deserialize(rest)?;
            Ok(AppInfo::V1(raw))
        }
        VersionTagLoader::NewV1 { rest, .. } => {
            let raw = AppInfoV1::deserialize(rest)?;
            Ok(AppInfo::V1(raw))
        }
    }
}
