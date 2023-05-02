//! Spin doctor: check and automatically fix problems with Spin apps.
#![deny(missing_docs)]

use std::{fmt::Debug, fs, future::Future, path::PathBuf, pin::Pin, process::Command, sync::Arc};

use anyhow::{ensure, Context, Result};
use async_trait::async_trait;
use tokio::sync::Mutex;
use toml_edit::Document;

/// Diagnoses for app manifest format problems.
pub mod manifest;
/// Test helpers.
pub mod test;
/// Diagnoses for Wasm source problems.
pub mod wasm;

/// Configuration for an app to be checked for problems.
pub struct Checkup {
    manifest_path: PathBuf,
    diagnostics: Vec<Box<dyn BoxingDiagnostic>>,
}

impl Checkup {
    /// Return a new checkup for the app manifest at the given path.
    pub fn new(manifest_path: impl Into<PathBuf>) -> Self {
        let mut checkup = Self {
            manifest_path: manifest_path.into(),
            diagnostics: vec![],
        };
        checkup.add_diagnostic::<manifest::version::VersionDiagnostic>();
        checkup.add_diagnostic::<manifest::trigger::TriggerDiagnostic>();
        checkup.add_diagnostic::<wasm::missing::WasmMissingDiagnostic>();
        checkup
    }

    /// Add a detectable problem to this checkup.
    pub fn add_diagnostic<D: Diagnostic + Default + 'static>(&mut self) -> &mut Self {
        self.diagnostics.push(Box::<D>::default());
        self
    }

    fn patient(&self) -> Result<PatientApp> {
        let path = &self.manifest_path;
        ensure!(
            path.is_file(),
            "No Spin app manifest file found at {path:?}"
        );

        let contents = fs::read_to_string(path)
            .with_context(|| format!("Couldn't read Spin app manifest file at {path:?}"))?;

        let manifest_doc: Document = contents
            .parse()
            .with_context(|| format!("Couldn't parse manifest file at {path:?} as valid TOML"))?;

        Ok(PatientApp {
            manifest_path: path.into(),
            manifest_doc,
        })
    }

    /// Find problems with the configured app, calling the given closure with
    /// each problem found.
    pub async fn for_each_diagnosis<F>(&self, mut f: F) -> Result<usize>
    where
        F: for<'a> FnMut(
            Box<dyn Diagnosis + 'static>,
            &'a mut PatientApp,
        ) -> Pin<Box<dyn Future<Output = Result<()>> + 'a>>,
    {
        let patient = Arc::new(Mutex::new(self.patient()?));
        let mut count = 0;
        for diagnostic in &self.diagnostics {
            let patient = patient.clone();
            let diags = diagnostic
                .diagnose_boxed(&*patient.lock().await)
                .await
                .unwrap_or_else(|err| {
                    tracing::debug!("Diagnose failed: {err:?}");
                    vec![]
                });
            count += diags.len();
            for diag in diags {
                let mut patient = patient.lock().await;
                f(diag, &mut patient).await?;
            }
        }
        Ok(count)
    }
}

/// An app "patient" to be checked for problems.
#[derive(Clone)]
pub struct PatientApp {
    /// Path to an app manifest file.
    pub manifest_path: PathBuf,
    /// Parsed app manifest TOML document.
    pub manifest_doc: Document,
}

/// The Diagnose trait implements the detection of a particular Spin app problem.
#[async_trait]
pub trait Diagnostic: Send + Sync {
    /// A [`Diagnosis`] representing the problem(s) this can detect.
    type Diagnosis: Diagnosis;

    /// Check the given [`Patient`], returning any problem(s) found.
    ///
    /// If multiple _independently addressable_ problems are found, this may
    /// return multiple instances. If two "logically separate" problems would
    /// have the same fix, they should be represented with the same instance.
    async fn diagnose(&self, patient: &PatientApp) -> Result<Vec<Self::Diagnosis>>;
}

/// The Diagnosis trait represents a detected problem with a Spin app.
pub trait Diagnosis: Debug + Send + Sync + 'static {
    /// Return a human-friendly description of this problem.
    fn description(&self) -> String;

    /// Return true if this problem is "critical", i.e. if the app's
    /// configuration or environment is invalid. Return false for
    /// "non-critical" problems like deprecations.
    fn is_critical(&self) -> bool {
        true
    }

    /// Return a [`Treatment`] that can (potentially) fix this problem, or
    /// None if there is no automatic fix.
    fn treatment(&self) -> Option<&dyn Treatment> {
        None
    }
}

/// The Treatment trait represents a (potential) fix for a detected problem.
#[async_trait]
pub trait Treatment: Sync {
    /// Return a human-readable description of what this treatment will do to
    /// fix the problem, such as a file diff.
    async fn description(&self, patient: &PatientApp) -> Result<String>;

    /// Attempt to fix this problem. Return Ok only if the problem is
    /// successfully fixed.
    async fn treat(&self, patient: &mut PatientApp) -> Result<()>;
}

const SPIN_BIN_PATH: &str = "SPIN_BIN_PATH";

/// Return a [`Command`] targeting the `spin` binary. The `spin` path is
/// resolved to the first of these that is available:
/// - the `SPIN_BIN_PATH` environment variable
/// - the current executable ([`std::env::current_exe`])
/// - the constant `"spin"` (resolved by e.g. `$PATH`)
pub fn spin_command() -> Command {
    let spin_path = std::env::var_os(SPIN_BIN_PATH)
        .map(PathBuf::from)
        .or_else(|| std::env::current_exe().ok())
        .unwrap_or("spin".into());
    Command::new(spin_path)
}

#[async_trait]
trait BoxingDiagnostic {
    async fn diagnose_boxed(&self, patient: &PatientApp) -> Result<Vec<Box<dyn Diagnosis>>>;
}

#[async_trait]
impl<Factory: Diagnostic> BoxingDiagnostic for Factory {
    async fn diagnose_boxed(&self, patient: &PatientApp) -> Result<Vec<Box<dyn Diagnosis>>> {
        Ok(self
            .diagnose(patient)
            .await?
            .into_iter()
            .map(|diag| Box::new(diag) as Box<dyn Diagnosis>)
            .collect())
    }
}
