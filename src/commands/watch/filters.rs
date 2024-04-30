use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Context;
use async_trait::async_trait;
use spin_common::ui::quoted_path;
use spin_manifest::schema::v2;

#[async_trait]
pub(crate) trait FilterFactory: Send + Sync {
    async fn build_filter(
        &self,
        manifest_file: &Path,
        manifest_dir: &Path,
        manifest: &v2::AppManifest,
    ) -> anyhow::Result<Arc<watchexec_filterer_globset::GlobsetFilterer>>;
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
    ) -> anyhow::Result<Arc<watchexec_filterer_globset::GlobsetFilterer>> {
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
            .map(|s| (s, None))
            .collect::<Vec<_>>();

        let filterer = watchexec_filterer_globset::GlobsetFilterer::new(
            manifest_dir,
            artifact_globs,
            standard_ignores(),
            [],
            [],
        )
        .await?;

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
    ) -> anyhow::Result<Arc<watchexec_filterer_globset::GlobsetFilterer>> {
        let manifest_glob = vec![manifest_path_to_watch(manifest_file)?];
        let src_globs = manifest
            .components
            .iter()
            .flat_map(|(cid, c)| create_source_globs(cid.as_ref(), c))
            .collect::<Vec<_>>();

        let build_globs = manifest_glob
            .into_iter()
            .chain(src_globs)
            .map(|s| (s, None))
            .collect::<Vec<_>>();

        let filterer = watchexec_filterer_globset::GlobsetFilterer::new(
            manifest_dir,
            build_globs,
            standard_ignores(),
            [],
            [],
        )
        .await?;

        Ok(Arc::new(filterer))
    }
}

fn create_source_globs(cid: &str, c: &v2::Component) -> Vec<String> {
    let Some(build) = &c.build else {
        return vec![];
    };
    if build.watch.is_empty() {
        eprintln!(
            "You haven't configured what to watch for the component: '{cid}'. Learn how to configure Spin watch at https://developer.fermyon.com/common/cli-reference#watch"
        );
        return vec![];
    };
    build
        .workdir
        .as_deref()
        .map(|workdir| {
            build
                .watch
                .iter()
                .filter_map(|w| Path::new(workdir).join(w).to_str().map(String::from))
                .collect()
        })
        .unwrap_or_else(|| build.watch.clone())
}

#[async_trait]
impl FilterFactory for ManifestFilterFactory {
    async fn build_filter(
        &self,
        manifest_file: &Path,
        manifest_dir: &Path,
        _: &v2::AppManifest,
    ) -> anyhow::Result<Arc<watchexec_filterer_globset::GlobsetFilterer>> {
        let manifest_glob = manifest_path_to_watch(manifest_file)?;

        let filterer = watchexec_filterer_globset::GlobsetFilterer::new(
            manifest_dir,
            vec![(manifest_glob, None)],
            standard_ignores(),
            [],
            [],
        )
        .await?;

        Ok(Arc::new(filterer))
    }
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
