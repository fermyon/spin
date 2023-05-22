/// Diagnose missing Wasm sources.
pub mod missing;

use std::path::Path;

use anyhow::Result;
use async_trait::async_trait;
use spin_loader::{local::config::RawComponentManifest, local::config::RawModuleSource};

use crate::{Diagnosis, Diagnostic, PatientApp};

/// PatientWasm represents a Wasm source to be checked for problems.
#[derive(Debug)]
pub struct PatientWasm(RawComponentManifest);

#[allow(missing_docs)] // WIP
impl PatientWasm {
    pub fn component_id(&self) -> &str {
        &self.0.id
    }

    pub fn source(&self) -> WasmSource {
        match &self.0.source {
            RawModuleSource::FileReference(path) => WasmSource::Local(path),
            _ => WasmSource::Other,
        }
    }

    pub fn has_build(&self) -> bool {
        self.0.build.is_some()
    }
}

/// WasmSource is a source (e.g. file path) of a Wasm binary.
#[derive(Debug)]
#[non_exhaustive]
pub enum WasmSource<'a> {
    /// Local file source path.
    Local(&'a Path),
    /// Other source (currently unsupported)
    Other,
}

/// WasmDiagnose helps implement [`Diagnose`] for Wasm source problems.
#[async_trait]
pub trait WasmDiagnostic {
    /// A [`Diagnosis`] representing the problem(s) this can detect.
    type Diagnosis: Diagnosis;

    /// The id of the diagnostic.
    const ID: &'static str;

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

    const ID: &'static str = T::ID;

    async fn diagnose(&self, patient: &PatientApp) -> Result<Vec<Self::Diagnosis>> {
        let path = &patient.manifest_path;
        let manifest = spin_loader::local::raw_manifest_from_file(&path)
            .await?
            .into_v1();
        let mut diagnoses = vec![];
        for component in manifest.components {
            let wasm = PatientWasm(component);
            diagnoses.extend(self.diagnose_wasm(patient, wasm).await?);
        }
        Ok(diagnoses)
    }
}
