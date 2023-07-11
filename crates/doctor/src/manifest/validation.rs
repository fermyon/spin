use anyhow::Result;
use async_trait::async_trait;
use spin_loader::local::validate_raw_app_manifest;

use crate::{Diagnosis, Diagnostic, PatientApp};

/// ValidationDiagnostic detects problems with "normal" app manifest validation.
#[derive(Default)]
pub struct ValidationDiagnostic;

#[async_trait]
impl Diagnostic for ValidationDiagnostic {
    type Diagnosis = ValidationDiagnosis;

    async fn diagnose(&self, patient: &PatientApp) -> Result<Vec<Self::Diagnosis>> {
        let raw_manifest = toml_edit::de::from_document(patient.manifest_doc.clone())?;
        if let Err(err) = validate_raw_app_manifest(&raw_manifest) {
            return Ok(vec![ValidationDiagnosis(err)]);
        }
        Ok(vec![])
    }
}

/// ValidationDiagnosis represents a problem with the app manifest validation.
#[derive(Debug)]
pub struct ValidationDiagnosis(anyhow::Error);

impl Diagnosis for ValidationDiagnosis {
    fn description(&self) -> String {
        format!("validation error: {}", self.0)
    }
}
