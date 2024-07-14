use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Context;
use async_trait::async_trait;
use spin_common::ui::quoted_path;
use spin_manifest::schema::v2;
use watchexec::filter::Filterer;

#[async_trait]
pub(crate) trait FilterFactory: Send + Sync {
    async fn build_filter(
        &self,
        manifest_file: &Path,
        manifest_dir: &Path,
        manifest: &v2::AppManifest,
    ) -> anyhow::Result<Arc<dyn Filterer>>;
}

pub(crate) struct ArtifactFilterFactory {
    pub skip_build: bool,
    pub skip_assets: bool,
}

pub(crate) struct BuildFilterFactory;
pub(crate) struct ManifestFilterFactory;

#[async_trait]
impl FilterFactory for ArtifactFilterFactory {
    async fn build_filter(
        &self,
        manifest_file: &Path,
        manifest_dir: &Path,
        manifest: &v2::AppManifest,
    ) -> anyhow::Result<Arc<dyn Filterer>> {
        let manifest_glob = if self.skip_build {
            vec![manifest_path_to_watch(manifest_file)?]
        } else {
            vec![] // In this case, manifest changes trigger a rebuild, which will poke the uppificator anyway
        };
        let wasm_globs = manifest
            .components
            .values()
            .filter_map(|c| match &c.source {
                v2::ComponentSource::Local(path) => Some(path.clone()),
                _ => None,
            });
        let asset_globs = match self.skip_assets {
            true => {
                tracing::debug!("Skipping asset globs from being watched");
                vec![]
            }
            false => manifest
                .components
                .values()
                .flat_map(|c| c.files.iter())
                .filter_map(globbify)
                .collect::<Vec<_>>(),
        };

        let artifact_globs = manifest_glob
            .into_iter()
            .chain(wasm_globs)
            .chain(asset_globs)
            .collect::<Vec<_>>();

        let filterer = globset_filter(manifest_dir, artifact_globs).await?;

        Ok(Arc::new(filterer))
    }
}

fn globbify(files_mount: &v2::WasiFilesMount) -> Option<String> {
    match files_mount {
        v2::WasiFilesMount::Placement { source, .. } => {
            Path::new(source).join("**/*").to_str().map(String::from)
        }
        v2::WasiFilesMount::Pattern(pattern) => Some(pattern.clone()),
    }
}

#[async_trait]
impl FilterFactory for BuildFilterFactory {
    async fn build_filter(
        &self,
        manifest_file: &Path,
        manifest_dir: &Path,
        manifest: &v2::AppManifest,
    ) -> anyhow::Result<Arc<dyn Filterer>> {
        let mut filterers: Vec<Box<dyn Filterer>> =
            Vec::with_capacity(manifest.components.len() + 1);

        let manifest_globs = vec![manifest_path_to_watch(manifest_file)?];
        let manifest_filterer = globset_filter(manifest_dir, manifest_globs).await?;

        filterers.push(Box::new(manifest_filterer));

        for (cid, c) in &manifest.components {
            if let Some(build_globs) = create_source_globs(cid.as_ref(), c) {
                let build_filterer = globset_filter(manifest_dir, build_globs).await?;
                filterers.push(Box::new(build_filterer));
            }
        }

        let filterer = CompositeFilterer { filterers };

        Ok(Arc::new(filterer))
    }
}

fn create_source_globs(cid: &str, c: &v2::Component) -> Option<Vec<String>> {
    let build = c.build.as_ref()?;
    if build.watch.is_empty() {
        eprintln!(
            "You haven't configured what to watch for the component: '{cid}'. Learn how to configure Spin watch at https://developer.fermyon.com/common/cli-reference#watch"
        );
        return None;
    };
    let globs = build
        .workdir
        .as_deref()
        .map(|workdir| {
            build
                .watch
                .iter()
                .filter_map(|w| Path::new(workdir).join(w).to_str().map(String::from))
                .collect()
        })
        .unwrap_or_else(|| build.watch.clone());
    if globs.is_empty() {
        // watchexec misinterprets empty list as "match all"
        None
    } else {
        Some(globs)
    }
}

#[async_trait]
impl FilterFactory for ManifestFilterFactory {
    async fn build_filter(
        &self,
        manifest_file: &Path,
        manifest_dir: &Path,
        _: &v2::AppManifest,
    ) -> anyhow::Result<Arc<dyn Filterer>> {
        let manifest_glob = manifest_path_to_watch(manifest_file)?;

        let filterer = globset_filter(manifest_dir, [manifest_glob]).await?;

        Ok(Arc::new(filterer))
    }
}

async fn globset_filter(
    manifest_dir: &Path,
    globs: impl IntoIterator<Item = String>,
) -> anyhow::Result<watchexec_filterer_globset::GlobsetFilterer> {
    let filterer = watchexec_filterer_globset::GlobsetFilterer::new(
        manifest_dir,
        globs.into_iter().map(|s| (s, None)),
        standard_ignores(),
        [],
        [],
    )
    .await?;

    Ok(filterer)
}

// Although manifest dir must be absolute, and most things are safer with abs
// file paths, the manifest _path_ for the watchers must be relative to manifest dir
fn manifest_path_to_watch(path: &Path) -> anyhow::Result<String> {
    let rel_path = path
        .file_name()
        .with_context(|| format!("resolved manifest {} has no filename", quoted_path(path)))?;
    Ok(rel_path.to_string_lossy().to_string())
}

fn standard_ignores() -> Vec<(String, Option<PathBuf>)> {
    [
        "**/*.swp", // Vim creates swap files during editing
    ]
    .into_iter()
    .map(|pat| (pat.to_owned(), None))
    .collect()
}

#[derive(Debug)]
struct CompositeFilterer {
    filterers: Vec<Box<dyn watchexec::filter::Filterer>>,
}

impl watchexec::filter::Filterer for CompositeFilterer {
    fn check_event(
        &self,
        event: &watchexec::event::Event,
        priority: watchexec::event::Priority,
    ) -> Result<bool, watchexec::error::RuntimeError> {
        // We are interested in a change if _any_ component is interested in it
        for f in &self.filterers {
            if f.check_event(event, priority)? {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
