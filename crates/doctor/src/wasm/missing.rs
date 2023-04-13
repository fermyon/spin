use std::process::Command;

use anyhow::{ensure, Result};
use async_trait::async_trait;

use crate::{spin_command, Diagnosis, PatientApp, Treatment};

use super::{PatientWasm, WasmDiagnose, WasmSource};

/// WasmMissing detects missing Wasm sources.
#[derive(Debug)]
pub struct WasmMissing(PatientWasm);

impl WasmMissing {
    fn build_cmd(&self, patient: &PatientApp) -> Result<Command> {
        let mut cmd = spin_command();
        cmd.arg("build")
            .arg("-f")
            .arg(&patient.manifest_path)
            .arg("--component-id")
            .arg(self.0.component_id());
        Ok(cmd)
    }
}

#[async_trait]
impl WasmDiagnose for WasmMissing {
    async fn diagnose_wasm(_app: &PatientApp, wasm: PatientWasm) -> anyhow::Result<Vec<Self>> {
        if let WasmSource::Local(path) = wasm.source() {
            if !path.exists() {
                return Ok(vec![Self(wasm)]);
            }
        }
        Ok(vec![])
    }
}

impl Diagnosis for WasmMissing {
    fn description(&self) -> String {
        let id = self.0.component_id();
        let WasmSource::Local(path) = self.0.source() else {
            unreachable!("unsupported source");
        };
        format!("Component {id:?} source {path:?} is missing")
    }

    fn treatment(&self) -> Option<&dyn Treatment> {
        self.0.has_build().then_some(self)
    }
}

#[async_trait]
impl Treatment for WasmMissing {
    async fn description(&self, patient: &PatientApp) -> anyhow::Result<String> {
        let args = self
            .build_cmd(patient)?
            .get_args()
            .map(|arg| arg.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ");
        Ok(format!("Run `spin {args}`"))
    }

    async fn treat(&self, patient: &mut PatientApp) -> anyhow::Result<()> {
        let mut cmd = self.build_cmd(patient)?;
        let status = cmd.status()?;
        ensure!(status.success(), "Build command {cmd:?} failed: {status:?}");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::test::{assert_single_diagnosis, TestPatient};

    use super::*;

    const MINIMUM_VIABLE_MANIFEST: &str = r#"
            spin_manifest_version = "1"
            name = "wasm-missing-test"
            version = "0.0.0"
            trigger = { type = "test" }
            [[component]]
            id = "missing-source"
            source = "does-not-exist.wasm"
            trigger = {}
        "#;

    #[tokio::test]
    async fn test_without_build() {
        let patient = TestPatient::from_toml_str(MINIMUM_VIABLE_MANIFEST);
        let diag = assert_single_diagnosis::<WasmMissing>(&patient).await;
        assert!(diag.treatment().is_none());
    }

    #[tokio::test]
    async fn test_with_build() {
        let manifest = format!("{MINIMUM_VIABLE_MANIFEST}\nbuild.command = 'true'");
        let patient = TestPatient::from_toml_str(manifest);
        let diag = assert_single_diagnosis::<WasmMissing>(&patient).await;
        assert!(diag.treatment().is_some());
        assert!(diag
            .build_cmd(&patient)
            .unwrap()
            .get_args()
            .any(|arg| arg == "missing-source"));
    }
}
