use std::path::PathBuf;

use spin_doctor::{Checkup, PatientDiagnosis};
use ui_testing::{Failed, UiTestsRunner};

fn main() -> anyhow::Result<()> {
    let mut runner = UiTestsRunner::default();
    for entry in glob::glob("tests/ui/**/*.toml")? {
        let path = entry?.canonicalize()?;
        let name = path.file_stem().unwrap().to_string_lossy();

        let test_path = path.clone();
        runner.add_async_test(
            format!("ui::{name}::diagnoses"),
            path.with_extension("diags"),
            move |_| run_diagnoses(test_path),
        );
        runner.add_async_test(
            format!("ui::{name}::treatments"),
            path.with_extension("cured"),
            move |_| run_treatments(path),
        );
    }
    runner.run_tests()
}

async fn run_diagnoses(path: PathBuf) -> Result<String, Failed> {
    let mut diags = vec![];
    let mut checkup = Checkup::new(path).expect("Checkup::new should work");
    while let Some(PatientDiagnosis { diagnosis, .. }) = checkup.next_diagnosis().await? {
        diags.push(diagnosis.description());
    }
    Ok(diags.join("\n"))
}

async fn run_treatments(path: PathBuf) -> Result<String, Failed> {
    let tempdir = tempfile::tempdir().expect("tempdir should work");
    let temp_path = tempdir.path().join("spin.toml");
    std::fs::copy(&path, &temp_path).expect("copy should work");
    let mut checkup = Checkup::new(&temp_path).expect("Checkup::new should work");
    while let Some(PatientDiagnosis { diagnosis, patient }) = checkup.next_diagnosis().await? {
        if let Some(treatment) = diagnosis.treatment() {
            treatment
                .treat(patient)
                .await
                .expect("treatment should work");
        }
    }
    Ok(checkup.patient().manifest_doc.to_string())
}
