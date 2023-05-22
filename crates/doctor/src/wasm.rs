/// Diagnose missing Wasm sources.
pub mod missing;

use std::path::{Path, PathBuf};

use anyhow::Result;
use async_trait::async_trait;
use spin_loader::{
    local::config::RawComponentManifest,
    local::{canonicalize_and_absolutize, config::RawModuleSource},
};

use crate::{Diagnosis, Diagnostic, PatientApp};

/// PatientWasm represents a Wasm source to be checked for problems.
#[derive(Debug)]
pub struct PatientWasm {
    app_dir: PathBuf,
    component: RawComponentManifest,
}

#[allow(missing_docs)] // WIP
impl PatientWasm {
    fn new(app_dir: impl AsRef<Path>, component: RawComponentManifest) -> Self {
        Self {
            app_dir: app_dir.as_ref().to_owned(),
            component,
        }
    }

    pub fn component_id(&self) -> &str {
        &self.component.id
    }

    pub fn source_path(&self) -> Option<&Path> {
        match &self.component.source {
            RawModuleSource::FileReference(path) => Some(path),
            _ => None,
        }
    }

    pub fn abs_source_path(&self) -> Option<PathBuf> {
        match &self.component.source {
            RawModuleSource::FileReference(path) => {
                // TODO: We probably need a doctor check to see if the path can be expanded!
                // For now, fall back to the literal path.
                let can_path = canonicalize_and_absolutize(path.clone(), &self.app_dir)
                    .unwrap_or(self.app_dir.join(path));
                Some(can_path)
            }
            _ => None,
        }
    }

    pub fn has_build(&self) -> bool {
        self.component.build.is_some()
    }
}

/// WasmDiagnose helps implement [`Diagnose`] for Wasm source problems.
#[async_trait]
pub trait WasmDiagnostic {
    /// A [`Diagnosis`] representing the problem(s) this can detect.
    type Diagnosis: Diagnosis;

    /// Check the given [`PatientWasm`], returning any problem(s) found.
    async fn diagnose_wasm(
        &self,
        app: &PatientApp,
        wasm: PatientWasm,
    ) -> Result<Vec<Self::Diagnosis>>;
}

#[async_trait]
impl<T: WasmDiagnostic + Send + Sync> Diagnostic for T {
    type Diagnosis = <Self as WasmDiagnostic>::Diagnosis;

    async fn diagnose(&self, patient: &PatientApp) -> Result<Vec<Self::Diagnosis>> {
        let path = &patient.manifest_path;
        let manifest = spin_loader::local::raw_manifest_from_file(&path)
            .await?
            .into_v1();
        let app_dir = path.parent().unwrap();
        let mut diagnoses = vec![];
        for component in manifest.components {
            let wasm = PatientWasm::new(app_dir, component);
            diagnoses.extend(self.diagnose_wasm(patient, wasm).await?);
        }
        Ok(diagnoses)
    }
}
