#![deny(missing_docs)]

use super::{
    bindle_writer::{self, ParcelSources},
    PublishError, PublishResult,
};
use bindle::{BindleSpec, Condition, Group, Invoice, Label, Parcel};
use semver::BuildMetadata;
use spin_loader::{
    bindle::config as bindle_schema,
    digest::{bytes_sha256_string, file_sha256_string},
    local::{absolutize, config as local_schema, parent_dir, validate_raw_app_manifest, UrlSource},
};
use std::path::{Path, PathBuf};

/// Expands a file-based application manifest to a Bindle invoice.
pub async fn expand_manifest(
    app_file: impl AsRef<Path>,
    buildinfo: Option<BuildMetadata>,
    scratch_dir: impl AsRef<Path>,
) -> PublishResult<(Invoice, ParcelSources)> {
    let app_file = absolutize(app_file)?;
    let manifest = spin_loader::local::raw_manifest_from_file(&app_file).await?;
    validate_raw_app_manifest(&manifest)?;
    let local_schema::RawAppManifestAnyVersion::V1(manifest) = manifest;
    let app_dir = parent_dir(&app_file)?;

    // * create a new spin.toml-like document where
    //   - each component changes its `files` entry to a group name
    //   - each component changes its `source` entry to a parcel SHA
    let dest_manifest = bindle_manifest(&manifest, &app_dir).await?;

    // * create an invoice where
    //   - the metadata is copied from the app manifest
    //   - there is a group for each component
    //   - there is a parcel for each asset
    //   - there is a parcel for each module source
    //   - if a component refers to an asset then the asset is in the component's group
    //     - the source and manifest parcels should NOT be group members
    //   - there is a parcel for the spin.toml-a-like and it has the magic media type

    // - n parcels for the Wasm modules at their locations
    let wasm_parcels = wasm_parcels(&manifest, &app_dir, &scratch_dir).await?;
    let wasm_parcels = consolidate_wasm_parcels(wasm_parcels);
    // - n parcels for the assets under the base directory
    let asset_parcels = asset_parcels(&manifest, &app_dir).await?;
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

async fn bindle_manifest(
    local: &local_schema::RawAppManifest,
    base_dir: &Path,
) -> PublishResult<bindle_schema::RawAppManifest> {
    let futures = local
        .components
        .iter()
        .map(|c| async { bindle_component_manifest(c, base_dir).await });
    let components = futures::future::join_all(futures)
        .await
        .into_iter()
        .collect::<PublishResult<Vec<_>>>()?;
    let trigger = local.info.trigger.clone();
    let variables = local.variables.clone();

    Ok(bindle_schema::RawAppManifest {
        trigger,
        components,
        variables,
    })
}

async fn bindle_component_manifest(
    local: &local_schema::RawComponentManifest,
    base_dir: &Path,
) -> PublishResult<bindle_schema::RawComponentManifest> {
    let source_digest = match &local.source {
        local_schema::RawModuleSource::FileReference(path) => {
            let full_path = base_dir.join(path);

            if let Ok(false) = Path::try_exists(&full_path) {
                return Err(PublishError::MissingBuildArtifact(
                    full_path.display().to_string(),
                ));
            }

            sha256_digest(&full_path)?
        }
        local_schema::RawModuleSource::Bindle(_) => {
            return Err(PublishError::BindlePushingNotImplemented);
        }
        local_schema::RawModuleSource::Url(us) => {
            let source = UrlSource::new(us)?;
            source.digest_str().to_owned()
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
    scratch_dir: impl AsRef<Path>,
) -> PublishResult<Vec<SourcedParcel>> {
    let parcel_futures = manifest
        .components
        .iter()
        .map(|c| wasm_parcel(c, base_dir, scratch_dir.as_ref()));
    let parcels = futures::future::join_all(parcel_futures).await;
    parcels.into_iter().collect()
}

async fn wasm_parcel(
    component: &local_schema::RawComponentManifest,
    base_dir: &Path,
    scratch_dir: impl AsRef<Path>,
) -> PublishResult<SourcedParcel> {
    let (wasm_file, absolute_wasm_file) = match &component.source {
        local_schema::RawModuleSource::FileReference(path) => {
            (path.to_owned(), base_dir.join(path))
        }
        local_schema::RawModuleSource::Bindle(_) => {
            return Err(PublishError::BindlePushingNotImplemented);
        }
        local_schema::RawModuleSource::Url(us) => {
            let source = UrlSource::new(us)?;
            let bytes = source.get().await?;
            let temp_dir = scratch_dir.as_ref().join("downloads");
            let absolute_path = write_file(&temp_dir, &us.digest.replace(':', "_"), &bytes).await?;
            let dest_relative_path = source.url_relative_path();

            (dest_relative_path, absolute_path)
        }
    };

    file_parcel(&absolute_wasm_file, wasm_file, None, "application/wasm").await
}

async fn asset_parcels(
    manifest: &local_schema::RawAppManifest,
    base_dir: impl AsRef<Path>,
) -> PublishResult<Vec<SourcedParcel>> {
    let assets_by_component: Vec<Vec<_>> = manifest
        .components
        .iter()
        .map(|c| collect_assets(c, &base_dir))
        .collect::<PublishResult<_>>()?;
    let parcel_futures = assets_by_component
        .iter()
        .flatten()
        .map(|(fm, s)| file_parcel_from_mount(fm, s));
    let parcel_results = futures::future::join_all(parcel_futures).await;
    let parcels = parcel_results.into_iter().collect::<PublishResult<_>>()?;
    Ok(parcels)
}

fn collect_assets(
    component: &local_schema::RawComponentManifest,
    base_dir: impl AsRef<Path>,
) -> PublishResult<Vec<(spin_loader::local::assets::FileMount, String)>> {
    let patterns = component.wasm.files.clone().unwrap_or_default();
    let exclude_files = component.wasm.exclude_files.clone().unwrap_or_default();
    let file_mounts = spin_loader::local::assets::collect(&patterns, &exclude_files, &base_dir)?;
    let annotated = file_mounts
        .into_iter()
        .map(|v| (v, component.id.clone()))
        .collect();
    Ok(annotated)
}

async fn file_parcel_from_mount(
    file_mount: &spin_loader::local::assets::FileMount,
    component_id: &str,
) -> PublishResult<SourcedParcel> {
    let source_file = &file_mount.src;

    let media_type = mime_guess::from_path(source_file)
        .first_or_octet_stream()
        .to_string();

    file_parcel(
        source_file,
        &file_mount.relative_dst,
        Some(component_id),
        &media_type,
    )
    .await
}

async fn file_parcel(
    abs_src: &Path,
    dest_relative_path: impl AsRef<Path>,
    component_id: Option<&str>,
    media_type: impl Into<String>,
) -> PublishResult<SourcedParcel> {
    let parcel = Parcel {
        label: Label {
            sha256: sha256_digest(abs_src)?,
            name: dest_relative_path.as_ref().display().to_string(),
            size: file_metadata(&abs_src).await?.len(),
            media_type: media_type.into(),
            annotations: None,
            feature: None,
            origin: None,
        },
        conditions: Some(Condition {
            member_of: component_id.map(|id| vec![group_name_for(id)]),
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
) -> PublishResult<SourcedParcel> {
    let text = toml::to_string_pretty(&manifest).map_err(|e| PublishError::TomlSerialization {
        source: e,
        description: "App manifest serialization failure".to_string(),
    })?;
    let bytes = text.as_bytes();
    let digest = bytes_sha256_string(bytes);
    let parcel_name = format!("spin.{}.toml", digest);
    let temp_dir = scratch_dir.as_ref().join("manifests");
    let absolute_path = write_file(&temp_dir, &parcel_name, bytes).await?;

    let parcel = Parcel {
        label: Label {
            sha256: digest.clone(),
            name: parcel_name,
            size: file_metadata(&absolute_path).await?.len(),
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

fn bindle_id(
    app_info: &local_schema::RawAppInformation,
    buildinfo: Option<BuildMetadata>,
) -> PublishResult<bindle::Id> {
    check_safe_bindle_name(&app_info.name)?;
    let text = match buildinfo {
        None => format!("{}/{}", app_info.name, app_info.version),
        Some(buildinfo) => format!("{}/{}+{}", app_info.name, app_info.version, buildinfo),
    };
    bindle::Id::try_from(&text).map_err(|_| PublishError::BindleId(text))
}

// This is both slightly conservative and slightly loose. According to the spec,
// the / character should also be allowed, but currently that is not supported
// by Hippo (Spin issue 504). And the - character should not be allowed, but lots of
// our tests use it...!
lazy_static::lazy_static! {
    static ref SAFE_BINDLE_NAME: regex::Regex = regex::Regex::new("^[-_\\p{L}\\p{N}]+$").expect("Invalid name regex");
}

fn check_safe_bindle_name(name: &str) -> PublishResult<()> {
    if SAFE_BINDLE_NAME.is_match(name) {
        Ok(())
    } else {
        Err(PublishError::BindleNameInvalidChars(name.to_owned()))
    }
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

fn sha256_digest(file: impl AsRef<Path>) -> PublishResult<String> {
    file_sha256_string(&file).map_err(|e| PublishError::Io {
        source: e,
        description: format!(
            "Failed to calculate digest for '{}'",
            file.as_ref().display()
        ),
    })
}

async fn write_file(dir: &PathBuf, filename: &String, data: &[u8]) -> PublishResult<PathBuf> {
    let file = dir.join(filename);

    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| PublishError::Io {
            source: e,
            description: format!("Failed to create directory: '{}'", dir.display()),
        })?;

    tokio::fs::write(&file, &data)
        .await
        .map_err(|e| PublishError::Io {
            source: e,
            description: format!("Failed to write file: '{}'", file.display()),
        })?;

    dunce::canonicalize(&file).map_err(|e| PublishError::Io {
        source: e,
        description: format!("Failed to get absolute path: '{}'", file.display()),
    })
}

async fn file_metadata(file: impl AsRef<Path>) -> PublishResult<std::fs::Metadata> {
    tokio::fs::metadata(&file)
        .await
        .map_err(|e| PublishError::Io {
            source: e,
            description: format!("Failed to get file metadata: '{}'", file.as_ref().display()),
        })
}

#[cfg(test)]
mod test {
    use spin_loader::local::config::RawAppInformation;

    use super::*;

    fn app_info(name: &str) -> RawAppInformation {
        app_info_v(name, "0.0.1")
    }

    fn app_info_v(name: &str, version: &str) -> RawAppInformation {
        RawAppInformation {
            name: name.to_owned(),
            version: version.to_owned(),
            description: None,
            authors: None,
            trigger: spin_manifest::ApplicationTrigger::Http(
                spin_manifest::HttpTriggerConfiguration {
                    base: "/".to_owned(),
                },
            ),
            namespace: None,
        }
    }

    #[test]
    fn accepts_only_valid_bindle_names() {
        bindle_id(&app_info("hello"), None).expect("should have accepted 'hello'");
        bindle_id(&app_info("hello-world"), None).expect("should have accepted 'hello-world'");
        bindle_id(&app_info("hello_world"), None).expect("should have accepted 'hello_world'");

        let err = bindle_id(&app_info("hello/world"), None)
            .expect_err("should not have accepted 'hello/world'");
        assert!(matches!(err, PublishError::BindleNameInvalidChars(_)));

        let err = bindle_id(&app_info("hello world"), None)
            .expect_err("should not have accepted 'hello world'");
        assert!(matches!(err, PublishError::BindleNameInvalidChars(_)));

        let err = bindle_id(&app_info_v("hello", "lolsnort"), None)
            .expect_err("should not have accepted version 'lolsnort'");
        assert!(matches!(err, PublishError::BindleId(_)));
    }
}
