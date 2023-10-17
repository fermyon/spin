use std::path::{Path, PathBuf};

use anyhow::{bail, ensure, Context, Result};
use futures::future::try_join_all;
use reqwest::Url;
use spin_app::{
    locked::{
        self, ContentPath, ContentRef, LockedApp, LockedComponent, LockedComponentSource,
        LockedTrigger,
    },
    values::{ValuesMap, ValuesMapBuilder},
};
use spin_common::paths::parent_dir;
use spin_manifest::schema::v2::{self, AppManifest, KebabId, WasiFilesMount};
use tokio::{fs, sync::Semaphore};

use crate::{cache::Cache, http::verified_download, FilesMountStrategy};

#[derive(Debug)]
pub struct LocalLoader {
    app_root: PathBuf,
    files_mount_strategy: FilesMountStrategy,
    cache: Cache,
    file_loading_permits: Semaphore,
}

impl LocalLoader {
    pub async fn new(app_root: &Path, files_mount_strategy: FilesMountStrategy) -> Result<Self> {
        let app_root = app_root
            .canonicalize()
            .with_context(|| format!("Invalid manifest dir `{}`", app_root.display()))?;
        Ok(Self {
            app_root,
            files_mount_strategy,
            cache: Cache::new(None).await?,
            // Limit concurrency to avoid hitting system resource limits
            file_loading_permits: Semaphore::new(crate::MAX_FILE_LOADING_CONCURRENCY),
        })
    }

    // Load the manifest file (spin.toml) at the given path into a LockedApp,
    // preparing all its content for execution.
    pub async fn load_file(&self, path: impl AsRef<Path>) -> Result<LockedApp> {
        // Parse manifest
        let path = path.as_ref();
        let manifest = spin_manifest::manifest_from_file(path)
            .with_context(|| format!("Failed to read Spin app manifest from {path:?}"))?;
        let mut locked = self
            .load_manifest(manifest)
            .await
            .with_context(|| format!("Failed to load Spin app from {path:?}"))?;

        // Set origin metadata
        locked
            .metadata
            .insert("origin".into(), file_url(path)?.into());

        Ok(locked)
    }

    // Load the given manifest into a LockedApp, ready for execution.
    async fn load_manifest(&self, mut manifest: AppManifest) -> Result<LockedApp> {
        spin_manifest::normalize::normalize_manifest(&mut manifest);

        let AppManifest {
            spin_manifest_version: _,
            application,
            variables,
            triggers,
            components,
        } = manifest;

        let metadata = locked_metadata(application, triggers.keys().cloned())?;

        let variables = variables
            .into_iter()
            .map(|(name, v)| Ok((name.to_string(), locked_variable(v)?)))
            .collect::<Result<_>>()?;

        let triggers = triggers
            .into_iter()
            .flat_map(|(trigger_type, configs)| {
                configs
                    .into_iter()
                    .map(|trigger| locked_trigger(trigger_type.clone(), trigger))
                    .collect::<Vec<_>>()
            })
            .collect::<Result<Vec<_>>>()?;

        // Load all components concurrently
        let components = try_join_all(components.into_iter().map(|(id, c)| async move {
            self.load_component(&id, c)
                .await
                .with_context(|| format!("Failed to load component `{id}`"))
        }))
        .await?;

        Ok(LockedApp {
            spin_lock_version: Default::default(),
            metadata,
            variables,
            triggers,
            components,
        })
    }

    // Load the given component into a LockedComponent, ready for execution.
    async fn load_component(
        &self,
        id: &KebabId,
        component: v2::Component,
    ) -> Result<LockedComponent> {
        outbound_http::allowed_http_hosts::parse_allowed_http_hosts(&component.allowed_http_hosts)?;
        let metadata = ValuesMapBuilder::new()
            .string("description", component.description)
            .string_array("allowed_http_hosts", component.allowed_http_hosts)
            .string_array("key_value_stores", component.key_value_stores)
            .string_array("databases", component.sqlite_databases)
            .string_array("ai_models", component.ai_models)
            .serializable("build", component.build)?
            .take();

        let source = self
            .load_component_source(component.source.clone())
            .await
            .with_context(|| format!("Failed to load Wasm source {}", component.source))?;

        let env = component.environment.into_iter().collect();

        let files = if component.files.is_empty() {
            vec![]
        } else {
            match &self.files_mount_strategy {
                FilesMountStrategy::Copy(files_mount_root) => {
                    let component_mount_root = files_mount_root.join(id.as_ref());
                    // Copy mounted files into component mount root, concurrently
                    try_join_all(component.files.iter().map(|f| {
                        self.copy_file_mounts(f, &component_mount_root, &component.exclude_files)
                    }))
                    .await?;

                    // All component files (copies) are in `component_mount_root` now
                    vec![ContentPath {
                        content: file_content_ref(component_mount_root)?,
                        path: "/".into(),
                    }]
                }
                FilesMountStrategy::Direct => {
                    ensure!(
                        component.exclude_files.is_empty(),
                        "Cannot load a component with `exclude_files` using --direct-mounts"
                    );
                    let mut files = vec![];
                    for mount in &component.files {
                        // Validate (and canonicalize) direct mount directory
                        files.push(self.resolve_direct_mount(mount).await?);
                    }
                    files
                }
            }
        };

        let config = component
            .variables
            .into_iter()
            .map(|(k, v)| (k.into(), v))
            .collect();

        Ok(LockedComponent {
            id: id.as_ref().into(),
            metadata,
            source,
            env,
            files,
            config,
        })
    }

    // Load a Wasm source from the given ContentRef and update the source
    // URL with an absolute path to the content.
    async fn load_component_source(
        &self,
        source: v2::ComponentSource,
    ) -> Result<LockedComponentSource> {
        let content = match source {
            v2::ComponentSource::Local(path) => file_content_ref(self.app_root.join(path))?,
            v2::ComponentSource::Remote { url, digest } => {
                self.load_http_source(&url, &digest).await?
            }
        };
        Ok(LockedComponentSource {
            content_type: "application/wasm".into(),
            content,
        })
    }

    // Load a Wasm source from the given HTTP ContentRef source URL and
    // return a ContentRef an absolute path to the local copy.
    async fn load_http_source(&self, url: &str, digest: &str) -> Result<ContentRef> {
        ensure!(
            digest.starts_with("sha256:"),
            "invalid `digest` {digest:?}; must start with 'sha256:'"
        );
        let path = if let Ok(cached_path) = self.cache.wasm_file(digest) {
            cached_path
        } else {
            let _loading_permit = self.file_loading_permits.acquire().await?;

            let dest = self.cache.wasm_path(digest);
            verified_download(url, digest, &dest)
                .await
                .with_context(|| format!("Error fetching source URL {url:?}"))?;
            dest
        };
        file_content_ref(path)
    }

    // Copy content(s) from the given `mount`
    async fn copy_file_mounts(
        &self,
        mount: &WasiFilesMount,
        dest_root: &Path,
        exclude_files: &[String],
    ) -> Result<()> {
        match mount {
            WasiFilesMount::Pattern(pattern) => {
                self.copy_glob_or_path(pattern, dest_root, exclude_files)
                    .await
            }
            WasiFilesMount::Placement {
                source,
                destination,
            } => {
                let src = Path::new(source);
                let dest = dest_root.join(destination.trim_start_matches('/'));
                self.copy_file_or_directory(src, &dest, exclude_files).await
            }
        }
    }

    // Copy files matching glob pattern or single file/directory path.
    async fn copy_glob_or_path(
        &self,
        glob_or_path: &str,
        dest_root: &Path,
        exclude_files: &[String],
    ) -> Result<()> {
        let path = self.app_root.join(glob_or_path);
        if path.exists() {
            let dest = dest_root.join(glob_or_path);
            if path.is_dir() {
                // "single/dir"
                let pattern = path.join("**/*");
                self.copy_glob(&pattern, &self.app_root, &dest, exclude_files)
                    .await?;
            } else {
                // "single/file.txt"
                self.copy_single_file(&path, &dest).await?;
            }
        } else if looks_like_glob_pattern(glob_or_path) {
            // "glob/pattern/*"
            self.copy_glob(&path, &self.app_root, dest_root, exclude_files)
                .await?;
        } else {
            bail!("{glob_or_path:?} does not exist and doesn't appear to be a glob pattern");
        }
        Ok(())
    }

    // Copy a single file or entire directory from `src` to `dest`
    async fn copy_file_or_directory(
        &self,
        src: &Path,
        dest: &Path,
        exclude_files: &[String],
    ) -> Result<()> {
        let src_path = self.app_root.join(src);
        let meta = fs::metadata(&src_path)
            .await
            .with_context(|| format!("invalid file mount source {src:?}"))?;
        if meta.is_dir() {
            // { source = "host/dir", destination = "guest/dir" }
            let pattern = src_path.join("**/*");
            self.copy_glob(&pattern, &src_path, dest, exclude_files)
                .await?;
        } else {
            // { source = "host/file.txt", destination = "guest/file.txt" }
            self.copy_single_file(&src_path, dest).await?;
        }
        Ok(())
    }

    // Copy files matching glob `pattern` into `dest_root`.
    async fn copy_glob(
        &self,
        pattern: &Path,
        src_prefix: &Path,
        dest_root: &Path,
        exclude_files: &[String],
    ) -> Result<()> {
        let pattern = pattern
            .to_str()
            .with_context(|| format!("invalid (non-utf8) file pattern {pattern:?}"))?;

        let paths = glob::glob(pattern)
            .with_context(|| format!("Failed to resolve glob pattern {pattern:?}"))?;

        let exclude_patterns = exclude_files
            .iter()
            .map(|pattern| {
                glob::Pattern::new(pattern)
                    .with_context(|| format!("Invalid exclude_files glob pattern {pattern:?}"))
            })
            .collect::<Result<Vec<_>>>()?;

        for path_res in paths {
            let src = path_res?;
            if !src.is_file() {
                continue;
            }

            let app_root_path = src.strip_prefix(&self.app_root)?;
            if exclude_patterns
                .iter()
                .any(|pattern| pattern.matches_path(app_root_path))
            {
                tracing::debug!(
                    "File {app_root_path:?} excluded by exclude_files {exclude_files:?}"
                );
                continue;
            }

            let relative_path = src.strip_prefix(src_prefix)?;
            let dest = dest_root.join(relative_path);
            self.copy_single_file(&src, &dest).await?;
        }
        Ok(())
    }

    // Copy a single file from `src` to `dest`, creating parent directories.
    async fn copy_single_file(&self, src: &Path, dest: &Path) -> Result<()> {
        // Sanity checks: src is in app_root...
        src.strip_prefix(&self.app_root)?;
        // ...and dest is in the Copy root.
        if let FilesMountStrategy::Copy(files_mount_root) = &self.files_mount_strategy {
            dest.strip_prefix(files_mount_root)?;
        } else {
            unreachable!();
        }

        let _loading_permit = self.file_loading_permits.acquire().await?;
        let dest_parent = parent_dir(dest)?;
        fs::create_dir_all(&dest_parent)
            .await
            .with_context(|| format!("Failed to create parent directory {dest_parent:?}"))?;
        fs::copy(src, dest)
            .await
            .with_context(|| format!("Failed to copy {src:?} to {dest:?}"))?;
        tracing::debug!("Copied {src:?} to {dest:?}");
        Ok(())
    }

    // Resolve the given direct mount directory, checking that it is valid for
    // direct mounting and returning its canonicalized source path.
    async fn resolve_direct_mount(&self, mount: &WasiFilesMount) -> Result<ContentPath> {
        let (src, dest) = match mount {
            WasiFilesMount::Pattern(pattern) => (pattern, pattern),
            WasiFilesMount::Placement {
                source,
                destination,
            } => (source, destination),
        };
        let path = self.app_root.join(src);
        if !path.is_dir() {
            bail!("Only directory mounts are supported with `--direct-mounts`; {src:?} is not a directory.");
        }
        Ok(ContentPath {
            content: file_content_ref(src)?,
            path: dest.into(),
        })
    }
}

fn locked_metadata(
    details: v2::AppDetails,
    trigger_types: impl Iterator<Item = String>,
) -> Result<ValuesMap> {
    let mut builder = ValuesMapBuilder::new();
    builder
        .string("name", details.name)
        .string("version", details.version)
        .string("description", details.description)
        .string_array("authors", details.authors)
        .serializable("triggers", &details.trigger_global_configs)?;

    // Duplicate single-trigger global options into "trigger" with "type"
    // key to maintain backward compatibility for a while.
    let types = trigger_types.collect::<Vec<_>>();
    if types.len() == 1 {
        let trigger_type = types.into_iter().next().unwrap();
        let mut single_trigger = details
            .trigger_global_configs
            .get(&trigger_type)
            .cloned()
            .unwrap_or_default();
        single_trigger.insert("type".into(), trigger_type.into());
        builder.serializable("trigger", single_trigger).unwrap();
    }

    Ok(builder.build())
}

fn locked_variable(variable: v2::Variable) -> Result<locked::Variable> {
    ensure!(
        variable.required ^ variable.default.is_some(),
        "must be `required` OR have a `default`"
    );
    Ok(locked::Variable {
        default: variable.default.clone(),
        secret: variable.secret,
    })
}

fn locked_trigger(trigger_type: String, trigger: v2::Trigger) -> Result<LockedTrigger> {
    fn reference_id(spec: v2::ComponentSpec) -> toml::Value {
        let v2::ComponentSpec::Reference(id) = spec else {
            unreachable!("should have already been normalized");
        };
        id.as_ref().into()
    }

    let mut config = trigger.config;
    if let Some(id) = trigger.component.map(reference_id) {
        config.insert("component".into(), id);
    }
    if !trigger.components.is_empty() {
        // Flatten trigger config `components` `OneOrManyComponentSpecs` into
        // lists of component references.
        config.insert(
            "components".into(),
            trigger
                .components
                .into_iter()
                .map(|(key, specs)| {
                    (
                        key,
                        specs
                            .0
                            .into_iter()
                            .map(reference_id)
                            .collect::<Vec<_>>()
                            .into(),
                    )
                })
                .collect::<toml::Table>()
                .into(),
        );
    }

    Ok(LockedTrigger {
        id: trigger.id,
        trigger_type,
        trigger_config: config.try_into()?,
    })
}

fn looks_like_glob_pattern(s: impl AsRef<str>) -> bool {
    let s = s.as_ref();
    glob::Pattern::escape(s) != s
}

fn file_content_ref(path: impl AsRef<Path>) -> Result<ContentRef> {
    Ok(ContentRef {
        source: Some(file_url(path)?),
        ..Default::default()
    })
}

fn file_url(path: impl AsRef<Path>) -> Result<String> {
    let path = path.as_ref();
    let abs_path = path
        .canonicalize()
        .with_context(|| format!("Couldn't resolve `{}`", path.display()))?;
    Ok(Url::from_file_path(abs_path).unwrap().to_string())
}
