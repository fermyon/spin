#![allow(dead_code)] // Refactor WIP

use std::path::PathBuf;

use anyhow::{ensure, Context, Result};
use async_trait::async_trait;
use spin_app::{
    locked::{ContentRef, LockedApp, LockedComponentSource},
    AppComponent, Loader,
};
use spin_core::StoreBuilder;
use spin_loader::cache::Cache;
use url::Url;

use crate::parse_file_url;

pub struct TriggerLoader {
    working_dir: PathBuf,
    allow_transient_write: bool,
}

impl TriggerLoader {
    pub fn new(working_dir: impl Into<PathBuf>, allow_transient_write: bool) -> Self {
        Self {
            working_dir: working_dir.into(),
            allow_transient_write,
        }
    }
}

#[async_trait]
impl Loader for TriggerLoader {
    async fn load_app(&self, url: &str) -> Result<LockedApp> {
        let path = parse_file_url(url)?;
        let contents =
            std::fs::read(&path).with_context(|| format!("failed to read manifest at {path:?}"))?;
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
        let path = parse_file_url(source)?;
        spin_core::Module::from_file(engine, &path)
            .with_context(|| format!("loading module {path:?}"))
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
            let source_path = self.working_dir.join(parse_file_url(source_uri)?);
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

pub struct OciTriggerLoader {
    working_dir: PathBuf,
    allow_transient_write: bool,
    cache: Cache,
}

impl OciTriggerLoader {
    // TODO: support a different cache root directory
    pub async fn new(
        working_dir: impl Into<PathBuf>,
        allow_transient_write: bool,
        cache_root: Option<PathBuf>,
    ) -> Result<Self> {
        Ok(Self {
            working_dir: working_dir.into(),
            allow_transient_write,
            cache: Cache::new(cache_root).await?,
        })
    }
}

#[async_trait]
impl Loader for OciTriggerLoader {
    // Read the locked app from the OCI cache and update the module source for each
    // component with the path to the Wasm modules from the OCI cache.
    async fn load_app(&self, url: &str) -> Result<LockedApp> {
        let path = parse_file_url(url)?;
        let contents =
            std::fs::read(&path).with_context(|| format!("failed to read manifest at {path:?}"))?;

        let app: LockedApp =
            serde_json::from_slice(&contents).context("failed to parse app lock file JSON")?;

        let mut res = app;
        let mut components = Vec::new();

        for mut c in res.components {
            let digest =
                c.clone().source.content.digest.expect(
                    "locked application from OCI cache should have a digest for the source",
                );

            let url = Url::from_file_path(self.cache.wasm_file(digest)?.to_str().unwrap())
                .expect("cannot crate file url from path for module source");

            c.source.content = ContentRef {
                digest: None,
                source: Some(url.to_string()),
            };

            components.push(c);
        }

        res.components = components;

        Ok(res)
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
        let path = parse_file_url(source)?;
        spin_core::Module::from_file(engine, &path)
            .with_context(|| format!("loading module {path:?}"))
    }

    // Copy static assets from the locked application into a temporary mount directory.
    async fn mount_files(
        &self,
        store_builder: &mut StoreBuilder,
        component: &AppComponent,
    ) -> Result<()> {
        let temp_mount = self.working_dir.join("files");
        tokio::fs::create_dir_all(&temp_mount)
            .await
            .context("cannot create temporary mount directory")?;

        for f in component.files() {
            let src = self
                .cache
                .data_file(f.clone().content.digest.context(format!(
                    "static asset {:?} from OCI cache must have a digest",
                    f
                ))?)?;
            let dst = temp_mount.join(&f.path);
            let parent = dst.parent().context(format!(
                "path for static asset mount path {:?} must have a parent directory",
                dst
            ))?;

            tokio::fs::create_dir_all(&parent)
                .await
                .context("cannot create directory structure for temporary mounts")?;
            tracing::trace!("Attempting to copy {:?}->{:?}", src, dst);
            tokio::fs::copy(&src, &dst)
                .await
                .context("cannot copy file mount")?;
        }

        if self.allow_transient_write {
            store_builder.read_write_preopened_dir(temp_mount, "/".into())?;
        } else {
            store_builder.read_only_preopened_dir(temp_mount, "/".into())?
        }

        Ok(())
    }
}
