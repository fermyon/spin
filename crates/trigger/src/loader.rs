use std::path::{Path, PathBuf};

use anyhow::{ensure, Context, Result};
use async_trait::async_trait;
use spin_app::{
    locked::{LockedApp, LockedComponentSource},
    AppComponent, Loader,
};
use spin_core::StoreBuilder;

pub struct TriggerLoader {
    working_dir: PathBuf,
    allow_transient_write: bool,
}

impl TriggerLoader {
    pub(crate) fn new(working_dir: impl Into<PathBuf>, allow_transient_write: bool) -> Self {
        Self {
            working_dir: working_dir.into(),
            allow_transient_write,
        }
    }
}

#[async_trait]
impl Loader for TriggerLoader {
    async fn load_app(&self, uri: &str) -> Result<LockedApp> {
        let path = unwrap_file_uri(uri)?;
        let contents =
            std::fs::read(path).with_context(|| format!("failed to read manifest at {path:?}"))?;
        let app =
            serde_json::from_slice(&contents).context("failed to parse app lock file JSON")?;
        Ok(app)
    }

    async fn load_module(
        &self,
        engine: &spin_core::wasmtime::Engine,
        source: &LockedComponentSource,
    ) -> Result<spin_core::Module> {
        let source = source
            .content
            .source
            .as_ref()
            .context("LockedComponentSource missing source field")?;
        let path = unwrap_file_uri(source)?;
        spin_core::Module::from_file(engine, path)
    }

    async fn mount_files(
        &self,
        store_builder: &mut StoreBuilder,
        component: &AppComponent,
    ) -> Result<()> {
        for content_dir in component.files() {
            let source_uri = content_dir
                .content
                .source
                .as_deref()
                .with_context(|| format!("Missing 'source' on files mount {content_dir:?}"))?;
            let source_path = self.working_dir.join(unwrap_file_uri(source_uri)?);
            ensure!(
                source_path.is_dir(),
                "TriggerLoader only supports directory mounts; {source_path:?} is not a directory"
            );
            let guest_path = content_dir.path.clone();
            if self.allow_transient_write {
                store_builder.read_write_preopened_dir(source_path, guest_path)?;
            } else {
                store_builder.read_only_preopened_dir(source_path, guest_path)?;
            }
        }
        Ok(())
    }
}

fn unwrap_file_uri(uri: &str) -> Result<&Path> {
    Ok(Path::new(
        uri.strip_prefix("file://")
            .context("TriggerLoader supports only file:// URIs")?,
    ))
}
