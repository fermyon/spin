use std::{ffi::OsStr, path::Path};

use anyhow::Context;
use futures::Future;
use libtest_mimic::{Arguments, Failed, Trial};
use snapbox::{Action, Assert, Data, Normalize};
use spin_loader::cache::Cache;

fn main() -> anyhow::Result<()> {
    let args = Arguments::from_args();

    // Insert dummy wasm into cache to avoid network traffic
    block_on(async {
        let cache = Cache::new(None).await.expect("Cache::new to work");
        cache
            .write_wasm(b"", "0000")
            .await
            .expect("write_wasms to work");
    });

    let mut tests = vec![];
    let dir = Path::new("tests/ui").canonicalize()?;
    for entry in std::fs::read_dir(dir)? {
        let mut path = entry?.path();
        let test_name = path
            .file_stem()
            .context("file_stem")?
            .to_string_lossy()
            .to_string();

        // */spin.toml are tests too!
        if path.is_dir() {
            path = path.join("spin.toml");
            if !path.exists() {
                continue;
            }
        } else if path.extension() != Some(OsStr::new("toml")) {
            continue;
        }
        let snapshot_path = path.with_extension("lock");

        tests.push(Trial::test(format!("ui::{test_name}"), move || {
            run_test(&path, &snapshot_path)
        }));
    }

    let conclusion = libtest_mimic::run(&args, tests);
    if conclusion.has_failed() {
        eprintln!("Snapshot files can be automatically updated by re-running with BLESS=1");
    }
    conclusion.exit();
}

fn run_test(manifest_path: &Path, snapshot_path: &Path) -> Result<(), Failed> {
    let snapshot_path = snapshot_path.to_path_buf();
    let temp_dir = tempfile::tempdir()?;
    let temp_path = temp_dir.path().canonicalize()?;

    let result = block_on(async {
        let app = spin_loader::from_file(manifest_path, Some(&temp_path)).await?;
        let locked = spin_trigger::locked::build_locked_app(app, &temp_path)?;
        Ok(serde_json::to_string_pretty(&locked)?)
    })
    .map_err(|err: anyhow::Error| format!("{err:?}"));

    let normalize = NormalizeContentPaths::new(manifest_path, &temp_path)?;
    assert_snapshot(result, &snapshot_path, normalize)?;
    Ok(())
}

fn assert_snapshot(
    result: Result<impl Into<Data>, String>,
    snapshot_path: &Path,
    normalize: impl Normalize,
) -> Result<(), String> {
    // If BLESS env is set (non-empty), overwrite snapshot files
    let bless = !std::env::var_os("BLESS").unwrap_or_default().is_empty();
    let assert = Assert::new().action(if bless {
        Action::Overwrite
    } else {
        Action::Verify
    });

    let mut snapshot_path = snapshot_path.to_path_buf();
    let contents = match result {
        Ok(data) => data.into(),
        Err(err) => {
            snapshot_path = snapshot_path.with_extension("err");
            if !bless && !snapshot_path.exists() {
                return Err(err);
            }
            err.into()
        }
    }
    .normalize(normalize);
    assert.eq_path(snapshot_path, contents);
    Ok(())
}

struct NormalizeContentPaths {
    root_dir: String,
    temp_dir: String,
}

impl NormalizeContentPaths {
    fn new(manifest_path: &Path, temp_dir: &Path) -> anyhow::Result<Self> {
        let root_dir = manifest_path
            .parent()
            .context("manifest_path parent")?
            .to_str()
            .context("root_dir to_str")?
            .to_string();
        let temp_dir = temp_dir.to_str().context("temp_dir to_str")?.to_string();
        Ok(Self { root_dir, temp_dir })
    }
}

impl Normalize for NormalizeContentPaths {
    fn normalize(&self, data: Data) -> Data {
        data.to_string()
            .replace(&self.root_dir, "<root-dir>")
            .replace(&self.temp_dir, "<temp-dir>")
            .into()
    }
}

fn block_on<T>(fut: impl Future<Output = T>) -> T {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime builder should work")
        .block_on(fut)
}
