use std::path::{Path, PathBuf};

use anyhow::{anyhow, ensure, Context, Result};
use oci_distribution::Reference;
use reqwest::Url;
use spin_app::locked::{ContentPath, ContentRef, LockedApp, LockedComponent};
use spin_loader::cache::Cache;

use crate::{Client, ORIGIN_URL_SCHEME};

/// OciLoader loads an OCI app in preparation for running with Spin.
pub struct OciLoader {
    working_dir: PathBuf,
}

impl OciLoader {
    /// Creates a new OciLoader which builds temporary mount directory(s) in
    /// the given working_dir.
    pub fn new(working_dir: impl Into<PathBuf>) -> Self {
        let working_dir = working_dir.into();
        Self { working_dir }
    }

    /// Loads a LockedApp with the given OCI client and reference.
    pub async fn load_app(&self, client: &mut Client, reference: &str) -> Result<LockedApp> {
        // Fetch app
        client.pull(reference).await.with_context(|| {
            format!("cannot pull Spin application from registry reference {reference:?}")
        })?;

        // Read locked app
        let lockfile_path = client
            .lockfile_path(&reference)
            .await
            .context("cannot get path to spin.lock")?;
        let locked_content = tokio::fs::read(&lockfile_path)
            .await
            .with_context(|| format!("failed to read from {lockfile_path:?}"))?;
        let mut locked_app = LockedApp::from_json(&locked_content)
            .with_context(|| format!("failed to decode locked app from {lockfile_path:?}"))?;

        // Update origin metadata
        let resolved_reference = Reference::try_from(reference).context("invalid reference")?;
        let origin_uri = format!("{ORIGIN_URL_SCHEME}:{resolved_reference}");
        locked_app
            .metadata
            .insert("origin".to_string(), origin_uri.into());

        for component in &mut locked_app.components {
            self.resolve_component_content_refs(component, &client.cache)
                .await
                .with_context(|| {
                    format!("failed to resolve content for component {:?}", component.id)
                })?;
        }
        Ok(locked_app)
    }

    async fn resolve_component_content_refs(
        &self,
        component: &mut LockedComponent,
        cache: &Cache,
    ) -> Result<()> {
        // Update wasm content path
        let wasm_digest = content_digest(&component.source.content)?;
        let wasm_path = cache.wasm_file(wasm_digest)?;
        component.source.content = content_ref(wasm_path)?;

        if !component.files.is_empty() {
            let mount_dir = self.working_dir.join(&component.id);
            for file in &mut component.files {
                let digest = content_digest(&file.content)?;
                let content_path = cache.data_file(digest)?;

                ensure!(is_safe_to_join(&file.path), "invalid file mount {file:?}");
                let mount_path = mount_dir.join(&file.path);

                // Create parent directory
                let mount_parent = mount_path
                    .parent()
                    .with_context(|| format!("invalid mount path {mount_path:?}"))?;
                tokio::fs::create_dir_all(mount_parent)
                    .await
                    .with_context(|| {
                        format!("failed to create temporary mount path {mount_path:?}")
                    })?;

                // Copy content
                // TODO: parallelize
                tokio::fs::copy(&content_path, &mount_path)
                    .await
                    .with_context(|| format!("failed to copy {content_path:?}->{mount_path:?}"))?;
            }

            component.files = vec![ContentPath {
                content: content_ref(mount_dir)?,
                path: "/".into(),
            }]
        }

        Ok(())
    }
}

fn content_digest(content_ref: &ContentRef) -> Result<&str> {
    content_ref
        .digest
        .as_deref()
        .with_context(|| format!("content missing expected digest: {content_ref:?}"))
}

fn content_ref(path: impl AsRef<Path>) -> Result<ContentRef> {
    let path = std::fs::canonicalize(path)?;
    let url = Url::from_file_path(path).map_err(|_| anyhow!("couldn't build file URL"))?;
    Ok(ContentRef {
        source: Some(url.to_string()),
        ..Default::default()
    })
}

fn is_safe_to_join(path: impl AsRef<Path>) -> bool {
    // This could be loosened, but currently should always be true
    path.as_ref()
        .components()
        .all(|c| matches!(c, std::path::Component::Normal(_)))
}
