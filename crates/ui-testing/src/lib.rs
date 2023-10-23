//! UI (aka "golden file") testing

use std::{
    future::Future,
    path::{Path, PathBuf},
};

use libtest_mimic::{Arguments, Trial};
use snapbox::{Action, Assert, Data, Normalize};

pub use libtest_mimic::Failed;

/// UI tests runner
#[derive(Default)]
pub struct UiTestsRunner {
    tests: Vec<Trial>,
}

impl UiTestsRunner {
    /// Adds a test to this runner.
    pub fn add_test<R>(
        &mut self,
        test_name: String,
        snapshot_path: impl Into<PathBuf>,
        runner: R,
    ) -> &mut Self
    where
        R: FnOnce(&mut Normalizer) -> Result<String, Failed> + Send + 'static,
    {
        let snapshot_path = snapshot_path.into();
        self.tests.push(Trial::test(test_name, move || {
            run_test(snapshot_path, runner)
        }));
        self
    }

    pub fn add_async_test<R, F>(
        &mut self,
        test_name: String,
        snapshot_path: impl Into<PathBuf>,
        runner: R,
    ) -> &mut Self
    where
        R: FnOnce(&mut Normalizer) -> F + Send + 'static,
        F: Future<Output = Result<String, Failed>>,
    {
        self.add_test(test_name, snapshot_path, |normalizer| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("tokio runtime builder should work")
                .block_on(runner(normalizer))
        })
    }

    /// For every entry in `tests/ui/*`, calls the path mapper to determine
    /// what path (if any) to use as a test input and expected "ok" output,
    /// then passes each input to the runner and compares the output against
    /// the expected "ok" output or a `.err` file matching the input entry.
    pub fn run_tests(&mut self) -> anyhow::Result<()> {
        let args = Arguments::from_args();
        let tests = std::mem::take(&mut self.tests);
        let conclusion = libtest_mimic::run(&args, tests);
        if conclusion.has_failed() {
            eprintln!("Snapshot files can be automatically updated by re-running with BLESS=1\n");
        }
        conclusion.exit_if_failed();
        Ok(())
    }
}

fn run_test<R>(snapshot_path: PathBuf, runner: R) -> Result<(), Failed>
where
    R: FnOnce(&mut Normalizer) -> Result<String, Failed> + Send + 'static,
{
    // If BLESS env is set (non-empty), overwrite snapshot files
    let bless = !std::env::var_os("BLESS").unwrap_or_default().is_empty();

    let snapshot_parent = snapshot_path.parent().unwrap().canonicalize().unwrap();
    let mut normalizer = Normalizer::new(&snapshot_parent);
    let result = runner(&mut normalizer);

    let (snapshot_path, snapshot_data) = match result {
        Ok(data) => (snapshot_path.to_path_buf(), data),
        Err(err) => {
            let snapshot_path = snapshot_path.with_extension("err");
            if !bless && !snapshot_path.exists() {
                return Err(err);
            }
            (snapshot_path, err.message().unwrap_or_default().to_string())
        }
    };
    let contents = Data::text(snapshot_data).normalize(normalizer);

    let assert = Assert::new().action(if bless {
        Action::Overwrite
    } else {
        Action::Verify
    });
    assert.eq_path(snapshot_path, contents);

    Ok(())
}

/// Normalizer configures test output normalization
pub struct Normalizer {
    replacements: Vec<(String, String)>,
}

impl Normalizer {
    fn new(snapshot_parent: &Path) -> Self {
        let mut n = Self {
            replacements: vec![],
        };
        n.replace_path(snapshot_parent, "<test-dir>");
        if let Some(cache_dir) = dirs::cache_dir() {
            n.replace_path(cache_dir, "<cache-dir>");
        }
        n
    }

    /// Configures a normalizing replacement for the given path.
    pub fn replace_path(&mut self, from: impl AsRef<Path>, to: impl Into<String>) -> &mut Self {
        let from = from
            .as_ref()
            .to_str()
            .expect("replace_path path must be utf-8");
        self.replace(from, to)
    }

    /// Configures a normalizing replacement for the given string.
    pub fn replace(&mut self, from: impl Into<String>, to: impl Into<String>) -> &mut Self {
        self.replacements.push((from.into(), to.into()));
        self
    }
}

impl Normalize for Normalizer {
    fn normalize(&self, data: Data) -> Data {
        let mut data = data.to_string();
        for (from, to) in &self.replacements {
            data = data.replace(from, to);
        }
        data.into()
    }
}
