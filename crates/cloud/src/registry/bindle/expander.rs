#![deny(missing_docs)]

use super::bindle_writer::{self, ParcelSources};
use anyhow::{Context, Result};
use bindle::{BindleSpec, Condition, Group, Invoice, Label, Parcel};
use path_absolutize::Absolutize;
use semver::BuildMetadata;
use sha2::{Digest, Sha256};
use spin_loader::{bindle::config as bindle_schema, local::config as local_schema};
use std::path::{Path, PathBuf};

/// Expands a file-based application manifest to a Bindle invoice.
pub async fn expand_manifest(
    app_file: impl AsRef<Path>,
    buildinfo: Option<BuildMetadata>,
    scratch_dir: impl AsRef<Path>,
) -> Result<(Invoice, ParcelSources)> {
    let app_file = app_file
        .as_ref()
        .absolutize()
        .context("Failed to resolve absolute path to manifest file")?;
    let manifest = spin_loader::local::raw_manifest_from_file(&app_file).await?;
    let local_schema::RawAppManifestAnyVersion::V1(manifest) = manifest;
    let app_dir = app_dir(&app_file)?;

    // * create a new spin.toml-like document where
    //   - each component changes its `files` entry to a group name
    //   - each component changes its `source` entry to a parcel SHA
    let dest_manifest = bindle_manifest(&manifest, &app_dir)?;

    // * create an invoice where
    //   - the metadata is copied from the app manifest
    //   - there is a group for each component
    //   - there is a parcel for each asset
    //   - there is a parcel for each module source
    //   - if a component refers to an asset then the asset is in the component's group
    //     - the source and manifest parcels should NOT be group members
    //   - there is a parcel for the spin.toml-a-like and it has the magic media type

    // - n parcels for the Wasm modules at their locations
    let wasm_parcels = wasm_parcels(&manifest, &app_dir)
        .await
        .context("Failed to collect Wasm modules")?;
    let wasm_parcels = consolidate_wasm_parcels(wasm_parcels);
    // - n parcels for the assets under the base directory
    let asset_parcels = asset_parcels(&manifest, &app_dir)
        .await
        .context("Failed to collect asset files")?;
    let asset_parcels = consolidate_asset_parcels(asset_parcels);
    // - one parcel to rule them all, and in the Spin app bind them
    let manifest_parcel = manifest_parcel(&dest_manifest, &scratch_dir).await?;

    let sourced_parcels = itertools::concat([vec![manifest_parcel], wasm_parcels, asset_parcels]);
    let (parcels, sources) = split_sources(sourced_parcels);

    let bindle_id = bindle_id(&manifest.info, buildinfo)?;
    let groups = build_groups(&manifest);

    let invoice = Invoice {
        bindle_version: "1.0.0".to_owned(),
        yanked: None,
        bindle: BindleSpec {
            id: bindle_id,
            description: manifest.info.description.clone(),
            authors: manifest.info.authors.clone(),
        },
        annotations: None,
        parcel: Some(parcels),
        group: Some(groups),
        signature: None,
        yanked_signature: None,
    };

    Ok((invoice, sources))
}

fn bindle_manifest(
    local: &local_schema::RawAppManifest,
    base_dir: &Path,
) -> Result<bindle_schema::RawAppManifest> {
    let components = local
        .components
        .iter()
        .map(|c| bindle_component_manifest(c, base_dir))
        .collect::<Result<Vec<_>>>()
        .context("Failed to convert components to Bindle format")?;
    let trigger = local.info.trigger.clone();
    let variables = local.variables.clone();

    Ok(bindle_schema::RawAppManifest {
        trigger,
        components,
        variables,
    })
}

fn bindle_component_manifest(
    local: &local_schema::RawComponentManifest,
    base_dir: &Path,
) -> Result<bindle_schema::RawComponentManifest> {
    let source_digest = match &local.source {
        local_schema::RawModuleSource::FileReference(path) => {
            let full_path = base_dir.join(path);
            file_digest_string(&full_path)
                .with_context(|| format!("Failed to get parcel id for '{}'", full_path.display()))?
        }
        local_schema::RawModuleSource::Bindle(_) => {
            anyhow::bail!(
                "This version of Spin can't publish components whose sources are already bindles"
            )
        }
    };
    let asset_group = local.wasm.files.as_ref().map(|_| group_name_for(&local.id));
    Ok(bindle_schema::RawComponentManifest {
        id: local.id.clone(),
        description: local.description.clone(),
        source: source_digest,
        wasm: bindle_schema::RawWasmConfig {
            environment: local.wasm.environment.clone(),
            files: asset_group,
            allowed_http_hosts: local.wasm.allowed_http_hosts.clone(),
        },
        trigger: local.trigger.clone(),
        config: local.config.clone(),
    })
}

async fn wasm_parcels(
    manifest: &local_schema::RawAppManifest,
    base_dir: &Path,
) -> Result<Vec<SourcedParcel>> {
    let parcel_futures = manifest.components.iter().map(|c| wasm_parcel(c, base_dir));
    let parcels = futures::future::join_all(parcel_futures).await;
    parcels.into_iter().collect()
}

async fn wasm_parcel(
    component: &local_schema::RawComponentManifest,
    base_dir: &Path,
) -> Result<SourcedParcel> {
    let wasm_file = match &component.source {
        local_schema::RawModuleSource::FileReference(path) => path,
        local_schema::RawModuleSource::Bindle(_) => {
            anyhow::bail!(
                "This version of Spin can't publish components whose sources are already bindles"
            )
        }
    };
    let absolute_wasm_file = base_dir.join(wasm_file);

    file_parcel(&absolute_wasm_file, wasm_file, None, "application/wasm").await
}

async fn asset_parcels(
    manifest: &local_schema::RawAppManifest,
    base_dir: impl AsRef<Path>,
) -> Result<Vec<SourcedParcel>> {
    let assets_by_component: Vec<Vec<_>> = manifest
        .components
        .iter()
        .map(|c| collect_assets(c, &base_dir))
        .collect::<Result<_>>()?;
    let parcel_futures = assets_by_component
        .iter()
        .flatten()
        .map(|(fm, s)| file_parcel_from_mount(fm, s));
    let parcel_results = futures::future::join_all(parcel_futures).await;
    let parcels = parcel_results.into_iter().collect::<Result<_>>()?;
    Ok(parcels)
}

fn collect_assets(
    component: &local_schema::RawComponentManifest,
    base_dir: impl AsRef<Path>,
) -> Result<Vec<(spin_loader::local::assets::FileMount, String)>> {
    let patterns = component.wasm.files.clone().unwrap_or_default();
    let exclude_files = component.wasm.exclude_files.clone().unwrap_or_default();
    let file_mounts = spin_loader::local::assets::collect(&patterns, &exclude_files, &base_dir)
        .with_context(|| format!("Failed to get file mounts for component '{}'", component.id))?;
    let annotated = file_mounts
        .into_iter()
        .map(|v| (v, component.id.clone()))
        .collect();
    Ok(annotated)
}

async fn file_parcel_from_mount(
    file_mount: &spin_loader::local::assets::FileMount,
    component_id: &str,
) -> Result<SourcedParcel> {
    let source_file = &file_mount.src;

    let media_type = mime_guess::from_path(&source_file)
        .first_or_octet_stream()
        .to_string();

    file_parcel(
        source_file,
        &file_mount.relative_dst,
        Some(component_id),
        &media_type,
    )
    .await
    .with_context(|| format!("Failed to assemble parcel from '{}'", source_file.display()))
}

async fn file_parcel(
    abs_src: &Path,
    dest_relative_path: impl AsRef<Path>,
    component_id: Option<&str>,
    media_type: impl Into<String>,
) -> Result<SourcedParcel> {
    let digest = file_digest_string(&abs_src)
        .with_context(|| format!("Failed to calculate digest for '{}'", abs_src.display()))?;
    let size = tokio::fs::metadata(&abs_src).await?.len();

    let member_of = component_id.map(|id| vec![group_name_for(id)]);

    let parcel = Parcel {
        label: Label {
            sha256: digest,
            name: dest_relative_path.as_ref().display().to_string(),
            size,
            media_type: media_type.into(),
            annotations: None,
            feature: None,
            origin: None,
        },
        conditions: Some(Condition {
            member_of,
            requires: None,
        }),
    };

    Ok(SourcedParcel {
        parcel,
        source: abs_src.to_owned(),
    })
}

async fn manifest_parcel(
    manifest: &bindle_schema::RawAppManifest,
    scratch_dir: impl AsRef<Path>,
) -> Result<SourcedParcel> {
    let text = toml::to_string_pretty(&manifest).context("Failed to write app manifest to TOML")?;
    let bytes = text.as_bytes();
    let digest = bytes_digest_string(bytes);

    let parcel_name = format!("spin.{}.toml", digest);
    let temp_dir = scratch_dir.as_ref().join("manifests");
    let temp_file = temp_dir.join(&parcel_name);

    tokio::fs::create_dir_all(temp_dir)
        .await
        .context("Failed to save app manifest to temporary file")?;
    tokio::fs::write(&temp_file, &bytes)
        .await
        .context("Failed to save app manifest to temporary file")?;

    let absolute_path = dunce::canonicalize(&temp_file)
        .context("Failed to acquire full path for app manifest temporary file")?;

    let parcel = Parcel {
        label: Label {
            sha256: digest.clone(),
            name: parcel_name,
            size: u64::try_from(bytes.len())?,
            media_type: spin_loader::bindle::SPIN_MANIFEST_MEDIA_TYPE.to_owned(),
            annotations: Some(bindle_writer::delete_after_copy()),
            feature: None,
            origin: None,
        },
        conditions: None,
    };

    Ok(SourcedParcel {
        parcel,
        source: absolute_path,
    })
}

fn consolidate_wasm_parcels(parcels: Vec<SourcedParcel>) -> Vec<SourcedParcel> {
    // We use only the content of Wasm parcels, not their names, so we only
    // care if the content is the same.
    let mut parcels = parcels;
    parcels.dedup_by_key(|p| p.parcel.label.sha256.clone());
    parcels
}

fn consolidate_asset_parcels(parcels: Vec<SourcedParcel>) -> Vec<SourcedParcel> {
    let mut consolidated = vec![];

    for mut parcel in parcels {
        match consolidated
            .iter_mut()
            .find(|p: &&mut SourcedParcel| can_consolidate_asset_parcels(&p.parcel, &parcel.parcel))
        {
            None => consolidated.push(parcel),
            Some(existing) => {
                // If can_consolidate returned true, both parcels must have conditions
                // and both conditions must have a member_of list.  So these unwraps
                // are safe.
                //
                // TODO: modify can_consolidate to return suitable stuff so we don't
                // have to unwrap.
                let existing_conds = existing.parcel.conditions.as_mut().unwrap();
                let conds_to_merge = parcel.parcel.conditions.as_mut().unwrap();
                let existing_member_of = existing_conds.member_of.as_mut().unwrap();
                let member_of_to_merge = conds_to_merge.member_of.as_mut().unwrap();
                existing_member_of.append(member_of_to_merge);
            }
        }
    }

    consolidated
}

fn can_consolidate_asset_parcels(first: &Parcel, second: &Parcel) -> bool {
    // For asset parcels, we care not only about the content, but where they
    // are placed and whether they have any metadata.  For example, if the same
    // image is needed both at /resources/logo.png and at /images/header.png,
    // we don't want to consolidate those references.
    if first.label.name == second.label.name
        && first.label.sha256 == second.label.sha256
        && first.label.size == second.label.size
        && first.label.media_type == second.label.media_type
        && first.label.annotations.is_none()
        && second.label.annotations.is_none()
        && first.label.feature.is_none()
        && second.label.feature.is_none()
    {
        match (&first.conditions, &second.conditions) {
            (Some(c1), Some(c2)) => {
                c1.member_of.is_some()
                    && c2.member_of.is_some()
                    && c1.requires.is_none()
                    && c2.requires.is_none()
            }
            _ => false,
        }
    } else {
        false
    }
}

fn build_groups(manifest: &local_schema::RawAppManifest) -> Vec<Group> {
    manifest
        .components
        .iter()
        .map(|c| group_for(&c.id))
        .collect()
}

fn group_name_for(component_id: &str) -> String {
    format!("files-{}", component_id)
}

fn group_for(component_id: &str) -> Group {
    Group {
        name: group_name_for(component_id),
        required: None,
        satisfied_by: None,
    }
}

fn file_digest_string(path: impl AsRef<Path>) -> Result<String> {
    let mut file = std::fs::File::open(&path)?;
    let mut sha = Sha256::new();
    std::io::copy(&mut file, &mut sha)?;
    let digest_value = sha.finalize();
    let digest_string = format!("{:x}", digest_value);
    Ok(digest_string)
}

fn bytes_digest_string(bytes: &[u8]) -> String {
    let digest_value = Sha256::digest(bytes);
    let digest_string = format!("{:x}", digest_value);
    digest_string
}

fn bindle_id(
    app_info: &local_schema::RawAppInformation,
    buildinfo: Option<BuildMetadata>,
) -> Result<bindle::Id> {
    let text = match buildinfo {
        None => format!("{}/{}", app_info.name, app_info.version),
        Some(buildinfo) => format!("{}/{}+{}", app_info.name, app_info.version, buildinfo),
    };
    bindle::Id::try_from(&text)
        .with_context(|| format!("App name and version '{}' do not form a bindle ID", text))
}

fn app_dir(app_file: impl AsRef<Path>) -> Result<std::path::PathBuf> {
    let path_buf = app_file
        .as_ref()
        .parent()
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Failed to get containing directory for app file '{}'",
                app_file.as_ref().display()
            )
        })?
        .to_owned();
    Ok(path_buf)
}

struct SourcedParcel {
    parcel: Parcel,
    source: PathBuf,
}

fn split_sources(sourced_parcels: Vec<SourcedParcel>) -> (Vec<Parcel>, ParcelSources) {
    let sources = sourced_parcels
        .iter()
        .map(|sp| (sp.parcel.label.sha256.clone(), &sp.source));
    let parcel_sources = ParcelSources::from_iter(sources);
    let parcels = sourced_parcels.into_iter().map(|sp| sp.parcel);

    (parcels.collect(), parcel_sources)
}
