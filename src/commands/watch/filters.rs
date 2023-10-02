use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::bail;
use async_trait::async_trait;
use spin_loader::local::config::{RawFileMount, RawModuleSource};

type Manifest = spin_loader::local::config::RawAppManifestImpl<spin_manifest::TriggerConfig>;
type Component = spin_loader::local::config::RawComponentManifestImpl<spin_manifest::TriggerConfig>;

#[async_trait]
pub(crate) trait FilterFactory: Send + Sync {
    async fn build_filter(
        &self,
        manifest_file: &Path,
        manifest_dir: &Path,
        manifest: &Manifest,
    ) -> anyhow::Result<Arc<watchexec_filterer_globset::GlobsetFilterer>>;
}

pub(crate) struct ArtifactFilterFactory {
    pub skip_build: bool,
}

pub(crate) struct BuildFilterFactory;
pub(crate) struct ManifestFilterFactory;

#[async_trait]
impl FilterFactory for ArtifactFilterFactory {
    async fn build_filter(
        &self,
        manifest_file: &Path,
        manifest_dir: &Path,
        manifest: &Manifest,
    ) -> anyhow::Result<Arc<watchexec_filterer_globset::GlobsetFilterer>> {
        let manifest_glob = if self.skip_build {
            vec![stringize_path(manifest_file)?]
        } else {
            vec![] // In this case, manifest changes trigger a rebuild, which will poke the uppificator anyway
        };
        let wasm_globs = manifest.components.iter().filter_map(|c| {
            let RawModuleSource::FileReference(path) = &c.source else {
                return None;
            };
            path.to_str().map(String::from)
        });
        let asset_globs = manifest
            .components
            .iter()
            .filter_map(|c| c.wasm.files.as_ref())
            .flatten()
            .filter_map(globbify)
            .collect::<Vec<_>>();

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

fn globbify(raw_file_mount: &RawFileMount) -> Option<String> {
    match raw_file_mount {
        RawFileMount::Placement(raw_directory_placement) => raw_directory_placement
            .source
            .join("**/*")
            .to_str()
            .map(String::from),
        RawFileMount::Pattern(pattern) => Some(pattern.to_string()),
    }
}

#[async_trait]
impl FilterFactory for BuildFilterFactory {
    async fn build_filter(
        &self,
        manifest_file: &Path,
        manifest_dir: &Path,
        manifest: &spin_loader::local::config::RawAppManifestImpl<spin_manifest::TriggerConfig>,
    ) -> anyhow::Result<Arc<watchexec_filterer_globset::GlobsetFilterer>> {
        let manifest_glob = vec![stringize_path(manifest_file)?];
        let src_globs = manifest
            .components
            .iter()
            .filter_map(create_source_globs)
            .flatten()
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

fn create_source_globs(c: &Component) -> Option<Vec<String>> {
    let build = c.build.as_ref()?;
    let Some(watch) = build.watch.clone() else {
        eprintln!(
            "You haven't configured what to watch for the component: '{}'. Learn how to configure Spin watch at https://developer.fermyon.com/common/cli-reference#watch",
            c.id
        );
        return None;
    };
    let sources = build
        .workdir
        .clone()
        .map(|workdir| {
            watch
                .iter()
                .filter_map(|w| workdir.join(w).to_str().map(String::from))
                .collect()
        })
        .unwrap_or(watch);
    Some(sources)
}

#[async_trait]
impl FilterFactory for ManifestFilterFactory {
    async fn build_filter(
        &self,
        manifest_file: &Path,
        manifest_dir: &Path,
        _: &spin_loader::local::config::RawAppManifestImpl<spin_manifest::TriggerConfig>,
    ) -> anyhow::Result<Arc<watchexec_filterer_globset::GlobsetFilterer>> {
        let manifest_glob = stringize_path(manifest_file)?;

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

fn stringize_path(path: &Path) -> anyhow::Result<String> {
    match path.to_str() {
        Some(s) => Ok(s.to_owned()),
        None => bail!("Can't represent path {} as string", path.display()),
    }
}

fn standard_ignores() -> Vec<(String, Option<PathBuf>)> {
    [
        "**/*.swp", // Vim creates swap files during editing
    ]
    .into_iter()
    .map(|pat| (pat.to_owned(), None))
    .collect()
}
