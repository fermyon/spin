//! Functionality to get a prepared Spin application configuration from spin.toml.

#![deny(missing_docs)]

/// Module to prepare the assets for the components of an application.
pub mod assets;
/// Configuration representation for a Spin application as a local spin.toml file.
pub mod config;

#[cfg(test)]
mod tests;

use std::{path::Path, sync::Arc};

use anyhow::{anyhow, bail, Context, Result};
use config::{RawAppInformation, RawAppManifest, RawAppManifestAnyVersion, RawComponentManifest};
use futures::future;
use path_absolutize::Absolutize;
use spin_manifest::{
    Application, ApplicationInformation, ApplicationOrigin, CoreComponent, ModuleSource,
    SpinVersion, WasmConfig,
};
use tokio::{fs::File, io::AsyncReadExt};

/// Given the path to a spin.toml manifest file, prepare its assets locally and
/// get a prepared application configuration consumable by a Spin execution context.
/// If a directory is provided, use it as the base directory to expand the assets,
/// otherwise create a new temporary directory.
pub async fn from_file(app: impl AsRef<Path>, base_dst: impl AsRef<Path>) -> Result<Application> {
    let app = app
        .as_ref()
        .absolutize()
        .context("Failed to resolve absolute path to manifest file")?;
    let manifest = raw_manifest_from_file(&app).await?;

    prepare_any_version(manifest, app, base_dst).await
}

/// Reads the spin.toml file as a raw manifest.
pub async fn raw_manifest_from_file(app: &impl AsRef<Path>) -> Result<RawAppManifestAnyVersion> {
    let mut buf = vec![];
    File::open(app.as_ref())
        .await?
        .read_to_end(&mut buf)
        .await
        .with_context(|| anyhow!("Cannot read manifest file from {:?}", app.as_ref()))?;

    let manifest: RawAppManifestAnyVersion = toml::from_slice(&buf)?;
    Ok(manifest)
}

/// Converts a raw application manifest into Spin configuration while handling
/// the Spin manifest and API version.
async fn prepare_any_version(
    raw: RawAppManifestAnyVersion,
    src: impl AsRef<Path>,
    base_dst: impl AsRef<Path>,
) -> Result<Application> {
    match raw {
        RawAppManifestAnyVersion::V1(raw) => prepare(raw, src, base_dst).await,
    }
}

/// Iterates over a vector of RawComponentManifest structs and throws an error if any component ids are duplicated
fn error_on_duplicate_ids(components: Vec<RawComponentManifest>) -> Result<()> {
    let mut ids: Vec<String> = Vec::new();
    for c in components {
        let id = c.id;
        if ids.contains(&id) {
            bail!("cannot have duplicate component IDs: {}", id);
        } else {
            ids.push(id);
        }
    }
    Ok(())
}

/// Converts a raw application manifest into Spin configuration.
async fn prepare(
    mut raw: RawAppManifest,
    src: impl AsRef<Path>,
    base_dst: impl AsRef<Path>,
) -> Result<Application> {
    let info = info(raw.info, &src);

    error_on_duplicate_ids(raw.components.clone())?;

    let mut config_root = raw.config.unwrap_or_default();
    for component in &mut raw.components {
        if let Some(config) = component.config.take() {
            let path = component.id.clone().try_into().with_context(|| {
                format!("component ID {:?} not a valid config path", component.id)
            })?;
            config_root.merge_defaults(&path, config)?;
        }
    }
    let config_resolver = Some(Arc::new(spin_config::Resolver::new(config_root)?));

    let component_triggers = raw
        .components
        .iter()
        .map(|c| (c.id.clone(), c.trigger.clone()))
        .collect();

    let components = future::join_all(
        raw.components
            .into_iter()
            .map(|c| async { core(c, &src, &base_dst).await })
            .collect::<Vec<_>>(),
    )
    .await
    .into_iter()
    .collect::<Result<Vec<_>>>()
    .context("Failed to prepare configuration")?;

    Ok(Application {
        info,
        components,
        component_triggers,
        config_resolver,
    })
}

/// Given a raw component manifest, prepare its assets and return a fully formed core component.
async fn core(
    raw: RawComponentManifest,
    src: impl AsRef<Path>,
    base_dst: impl AsRef<Path>,
) -> Result<CoreComponent> {
    let src = src
        .as_ref()
        .parent()
        .expect("The application file did not have a parent directory.");
    let source = match raw.source {
        config::RawModuleSource::FileReference(p) => {
            let p = match p.is_absolute() {
                true => p,
                false => src.join(p),
            };

            ModuleSource::FileReference(p)
        }
        config::RawModuleSource::Bindle(_) => {
            todo!("Bindle module sources are not yet supported in file-based app config")
        }
    };

    let id = raw.id;
    let description = raw.description;
    let mounts = match raw.wasm.files {
        Some(f) => assets::prepare_component(&f, src, &base_dst, &id).await?,
        None => vec![],
    };
    let environment = raw.wasm.environment.unwrap_or_default();
    let allowed_http_hosts = raw.wasm.allowed_http_hosts.unwrap_or_default();
    let wasm = WasmConfig {
        environment,
        mounts,
        allowed_http_hosts,
    };
    Ok(CoreComponent {
        source,
        id,
        description,
        wasm,
    })
}

/// Converts the raw application information from the spin.toml manifest to the standard configuration.
fn info(raw: RawAppInformation, src: impl AsRef<Path>) -> ApplicationInformation {
    ApplicationInformation {
        spin_version: SpinVersion::V1,
        name: raw.name,
        version: raw.version,
        description: raw.description,
        authors: raw.authors.unwrap_or_default(),
        trigger: raw.trigger,
        namespace: raw.namespace,
        origin: ApplicationOrigin::File(src.as_ref().to_path_buf()),
    }
}
