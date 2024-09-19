use std::fs;

use anyhow::{Context, Result};
use async_trait::async_trait;
use spin_common::ui::quoted_path;
use toml_edit::DocumentMut;

use crate::Treatment;

/// Diagnose app manifest trigger config problems.
pub mod trigger;
/// Diagnose old app manifest versions.
pub mod upgrade;
/// Diagnose upgradable app manifest versions.
pub mod version;

/// ManifestTreatment helps implement [`Treatment`]s for app manifest problems.
#[async_trait]
pub trait ManifestTreatment {
    /// Return a short (single line) description of what this fix will do, as
    /// an imperative, e.g. "Add default trigger config".
    fn summary(&self) -> String;

    /// Attempt to fix this problem. See [`Treatment::treat`].
    async fn treat_manifest(&self, doc: &mut DocumentMut) -> Result<()>;
}

#[async_trait]
impl<T: ManifestTreatment + Sync> Treatment for T {
    fn summary(&self) -> String {
        ManifestTreatment::summary(self)
    }

    async fn dry_run(&self, patient: &crate::PatientApp) -> Result<String> {
        let mut after_doc = patient.manifest_doc.clone();
        self.treat_manifest(&mut after_doc).await?;
        let before = patient.manifest_doc.to_string();
        let after = after_doc.to_string();
        let diff = similar::udiff::unified_diff(Default::default(), &before, &after, 1, None);
        Ok(format!(
            "Apply the following diff to {}:\n{}",
            quoted_path(&patient.manifest_path),
            diff
        ))
    }

    async fn treat(&self, patient: &mut crate::PatientApp) -> Result<()> {
        let doc = &mut patient.manifest_doc;
        self.treat_manifest(doc).await?;
        let path = &patient.manifest_path;
        fs::write(path, doc.to_string())
            .with_context(|| format!("failed to write fixed manifest to {}", quoted_path(path)))
    }
}
