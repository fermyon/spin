#![cfg(test)]
#![allow(clippy::expect_fun_call)]

use std::{fs, io::Write, path::Path};

use tempfile::{NamedTempFile, TempPath};
use toml::Value;

use super::*;

/// Asserts that the manifest at "tests/data/<prefix>_correct.toml" does
/// not have the given [`ManifestCondition`].
pub async fn run_correct_test<D: Diagnostic + Default>(prefix: &str) {
    let patient = TestPatient::from_file(test_file_path(prefix, "correct"));
    let diags = D::default()
        .diagnose(&patient)
        .await
        .expect("diagnose failed");
    assert!(diags.is_empty(), "expected correct file; got {diags:?}");
}

/// Asserts that the manifest at "tests/data/<prefix>_broken.toml" has
/// the given [`ManifestCondition`]. Also asserts that after fixing the
/// problem the manifest matches "tests/data/<prefix>_fixed.toml".
pub async fn run_broken_test<D: Diagnostic + Default>(prefix: &str, suffix: &str) -> D::Diagnosis {
    let mut patient = TestPatient::from_file(test_file_path(prefix, suffix));

    let diag = assert_single_diagnosis::<D>(&patient).await;
    let treatment = diag
        .treatment()
        .expect(&format!("{diag:?} should be treatable"));

    treatment
        .treat(&mut patient)
        .await
        .expect("treatment should succeed");

    let correct_path = test_file_path(prefix, "correct");
    let fixed_contents =
        fs::read_to_string(&correct_path).expect(&format!("reading {correct_path:?} failed"));
    assert_eq!(
        patient.manifest_doc.to_string().trim_end(),
        fixed_contents.trim_end()
    );

    diag
}

pub async fn assert_single_diagnosis<D: Diagnostic + Default>(
    patient: &PatientApp,
) -> D::Diagnosis {
    let diags = D::default()
        .diagnose(patient)
        .await
        .expect("diagnose should succeed");
    assert!(diags.len() == 1, "expected one diagnosis, got {diags:?}");
    diags.into_iter().next().unwrap()
}

fn test_file_path(prefix: &str, suffix: &str) -> PathBuf {
    format!("tests/data/{prefix}_{suffix}.toml").into()
}

pub struct TestPatient {
    inner: PatientApp,
    _manifest_temp: TempPath,
}

impl TestPatient {
    fn new(manifest_temp: TempPath) -> Result<Self> {
        let inner = PatientApp::new(&manifest_temp)?;
        Ok(Self {
            inner,
            _manifest_temp: manifest_temp,
        })
    }

    pub fn from_file(manifest_path: impl AsRef<Path>) -> Self {
        let manifest_temp = NamedTempFile::new()
            .expect("creating tempfile")
            .into_temp_path();

        let manifest_path = manifest_path.as_ref();
        std::fs::copy(manifest_path, &manifest_temp)
            .expect(&format!("copying {manifest_path:?} to tempfile"));

        Self::new(manifest_temp).expect(&format!("{manifest_path:?} should be a valid test file"))
    }

    pub fn from_toml(manifest: impl Into<Value>) -> Self {
        let mut manifest_file = NamedTempFile::new().expect("creating tempfile");
        let content = toml::to_string(&manifest.into()).expect("valid TOML");
        manifest_file
            .write_all(content.as_bytes())
            .expect("writing TOML");
        Self::new(manifest_file.into_temp_path()).unwrap()
    }

    pub fn from_toml_str(manifest: impl AsRef<str>) -> Self {
        Self::from_toml(toml::from_str::<Value>(manifest.as_ref()).expect("valid TOML"))
    }
}

impl std::ops::Deref for TestPatient {
    type Target = PatientApp;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl std::ops::DerefMut for TestPatient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
