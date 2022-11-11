//! Functionality to get a prepared Spin application configuration from spin.toml.

#![deny(missing_docs)]

/// Module to prepare the assets for the components of an application.
pub mod assets;
/// Configuration representation for a Spin application as a local spin.toml file.
pub mod config;

#[cfg(test)]
mod tests;

use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{anyhow, bail, Context, Result};
use futures::future;
use itertools::Itertools;
use outbound_http::allowed_http_hosts::validate_allowed_http_hosts;
use path_absolutize::Absolutize;
use reqwest::Url;
use spin_manifest::{
    Application, ApplicationInformation, ApplicationOrigin, CoreComponent, ModuleSource,
    SpinVersion, WasmConfig,
};
use tokio::{fs::File, io::AsyncReadExt};

use crate::{bindle::BindleConnectionInfo, digest::bytes_sha256_string};
use config::{RawAppInformation, RawAppManifest, RawAppManifestAnyVersion, RawComponentManifest};

use self::config::FileComponentUrlSource;

/// Given the path to a spin.toml manifest file, prepare its assets locally and
/// get a prepared application configuration consumable by a Spin execution context.
/// If a directory is provided, use it as the base directory to expand the assets,
/// otherwise create a new temporary directory.
pub async fn from_file(
    app: impl AsRef<Path>,
    base_dst: impl AsRef<Path>,
    bindle_connection: &Option<BindleConnectionInfo>,
) -> Result<Application> {
    let app = absolutize(app)?;
    let manifest = raw_manifest_from_file(&app).await?;
    validate_raw_app_manifest(&manifest)?;

    prepare_any_version(manifest, app, base_dst, bindle_connection).await
}

/// Reads the spin.toml file as a raw manifest.
pub async fn raw_manifest_from_file(app: &impl AsRef<Path>) -> Result<RawAppManifestAnyVersion> {
    let mut buf = vec![];
    File::open(app.as_ref())
        .await
        .with_context(|| anyhow!("Cannot read manifest file from {:?}", app.as_ref()))?
        .read_to_end(&mut buf)
        .await
        .with_context(|| anyhow!("Cannot read manifest file from {:?}", app.as_ref()))?;

    let manifest: RawAppManifestAnyVersion = toml::from_slice(&buf)
        .with_context(|| anyhow!("Cannot read manifest file from {:?}", app.as_ref()))?;

    Ok(manifest)
}

/// Returns the absolute path to directory containing the file
pub fn parent_dir(file: impl AsRef<Path>) -> Result<PathBuf> {
    let path_buf = file.as_ref().parent().ok_or_else(|| {
        anyhow::anyhow!(
            "Failed to get containing directory for file '{}'",
            file.as_ref().display()
        )
    })?;

    absolutize(path_buf)
}

/// Returns absolute path to the file
pub fn absolutize(path: impl AsRef<Path>) -> Result<PathBuf> {
    let path = path.as_ref();

    Ok(path
        .absolutize()
        .with_context(|| format!("Failed to resolve absolute path to: {}", path.display()))?
        .to_path_buf())
}

/// Converts a raw application manifest into Spin configuration while handling
/// the Spin manifest and API version.
async fn prepare_any_version(
    raw: RawAppManifestAnyVersion,
    src: impl AsRef<Path>,
    base_dst: impl AsRef<Path>,
    bindle_connection: &Option<BindleConnectionInfo>,
) -> Result<Application> {
    match raw {
        RawAppManifestAnyVersion::V1(raw) => prepare(raw, src, base_dst, bindle_connection).await,
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

/// Validate fields in raw app manifest
pub fn validate_raw_app_manifest(raw: &RawAppManifestAnyVersion) -> Result<()> {
    match raw {
        RawAppManifestAnyVersion::V1(raw) => {
            raw.components
                .iter()
                .try_for_each(|c| validate_allowed_http_hosts(&c.wasm.allowed_http_hosts))?;
        }
    }
    Ok(())
}

/// Converts a raw application manifest into Spin configuration.
async fn prepare(
    raw: RawAppManifest,
    src: impl AsRef<Path>,
    base_dst: impl AsRef<Path>,
    bindle_connection: &Option<BindleConnectionInfo>,
) -> Result<Application> {
    let info = info(raw.info, &src);

    error_on_duplicate_ids(raw.components.clone())?;

    let component_triggers = raw
        .components
        .iter()
        .map(|c| (c.id.clone(), c.trigger.clone()))
        .collect();

    let components = future::join_all(
        raw.components
            .into_iter()
            .map(|c| async { core(c, &src, &base_dst, bindle_connection).await })
            .collect::<Vec<_>>(),
    )
    .await
    .into_iter()
    .collect::<Result<Vec<_>>>()
    .context("Failed to prepare configuration")?;

    let variables = raw
        .variables
        .into_iter()
        .map(|(key, var)| Ok((key, var.try_into()?)))
        .collect::<Result<_>>()?;

    Ok(Application {
        info,
        variables,
        components,
        component_triggers,
    })
}

/// Given a raw component manifest, prepare its assets and return a fully formed core component.
async fn core(
    raw: RawComponentManifest,
    src: impl AsRef<Path>,
    base_dst: impl AsRef<Path>,
    bindle_connection: &Option<BindleConnectionInfo>,
) -> Result<CoreComponent> {
    let id = raw.id;
    let src = parent_dir(src)?;
    let source = match raw.source {
        config::RawModuleSource::FileReference(p) => {
            let p = match p.is_absolute() {
                true => p,
                false => src.join(p),
            };

            ModuleSource::FileReference(p)
        }
        config::RawModuleSource::Bindle(b) => {
            let bindle_id = bindle::Id::from_str(&b.reference).with_context(|| {
                format!("Invalid bindle ID {} in component {}", b.reference, id)
            })?;
            let parcel_sha = &b.parcel;
            let client = match bindle_connection {
                None => anyhow::bail!(
                    "Component {} requires a Bindle connection but none was specified",
                    id
                ),
                Some(c) => c.client()?,
            };
            let bindle_reader = crate::bindle::BindleReader::remote(&client, &bindle_id);
            let bytes = bindle_reader
                .get_parcel(parcel_sha)
                .await
                .with_context(|| {
                    format!(
                        "Failed to download parcel {}@{} for component {}",
                        bindle_id, parcel_sha, id
                    )
                })?;
            let name = format!("{}@{}", bindle_id, parcel_sha);
            ModuleSource::Buffer(bytes, name)
        }
        config::RawModuleSource::Url(us) => {
            let source = UrlSource::new(&us)
                .with_context(|| format!("Can't use Web source in component {}", id))?;

            let bytes = source
                .get()
                .await
                .with_context(|| format!("Can't use source {} for component {}", us.url, id))?;

            ModuleSource::Buffer(bytes, us.url)
        }
    };

    let description = raw.description;
    let mounts = match raw.wasm.files {
        Some(f) => {
            let exclude_files = raw.wasm.exclude_files.unwrap_or_default();
            assets::prepare_component(&f, src, &base_dst, &id, &exclude_files).await?
        }
        None => vec![],
    };
    let environment = raw.wasm.environment.unwrap_or_default();
    let allowed_http_hosts = raw.wasm.allowed_http_hosts.unwrap_or_default();
    let wasm = WasmConfig {
        environment,
        mounts,
        allowed_http_hosts,
    };
    let config = raw.config.unwrap_or_default();
    Ok(CoreComponent {
        source,
        id,
        description,
        wasm,
        config,
    })
}

/// A parsed URL source for a component module.
#[derive(Debug)]
pub struct UrlSource {
    url: Url,
    digest: ComponentDigest,
}

impl UrlSource {
    /// Parses a URL source from a raw component manifest.
    pub fn new(us: &FileComponentUrlSource) -> anyhow::Result<UrlSource> {
        let url = reqwest::Url::parse(&us.url)
            .with_context(|| format!("Invalid source URL {}", us.url))?;
        if url.scheme() != "https" {
            anyhow::bail!("Invalid URL scheme {}: must be HTTPS", url.scheme(),);
        }

        let digest = ComponentDigest::try_from(&us.digest)?;

        Ok(Self { url, digest })
    }

    /// The URL of the source.
    pub fn url(&self) -> &Url {
        &self.url
    }

    /// A relative path URL derived from the URL.
    pub fn url_relative_path(&self) -> PathBuf {
        let path = self.url.path();
        let rel_path = path.trim_start_matches('/');
        PathBuf::from(rel_path)
    }

    /// The digest string (omitting the format).
    pub fn digest_str(&self) -> &str {
        match &self.digest {
            ComponentDigest::Sha256(s) => s,
        }
    }

    /// Gets the data from the source as a byte buffer.
    pub async fn get(&self) -> anyhow::Result<Vec<u8>> {
        let response = reqwest::get(self.url.clone())
            .await
            .with_context(|| format!("Error fetching source URL {}", self.url))?;
        // TODO: handle redirects
        let status = response.status();
        if status != reqwest::StatusCode::OK {
            let reason = status.canonical_reason().unwrap_or("(no reason provided)");
            anyhow::bail!(
                "Error fetching source URL {}: {} {}",
                self.url,
                status.as_u16(),
                reason
            );
        }
        let body = response
            .bytes()
            .await
            .with_context(|| format!("Error loading source URL {}", self.url))?;
        let bytes = body.into_iter().collect_vec();

        self.digest.verify(&bytes).context("Incorrect digest")?;

        Ok(bytes)
    }
}

#[derive(Debug)]
enum ComponentDigest {
    Sha256(String),
}

impl TryFrom<&String> for ComponentDigest {
    type Error = anyhow::Error;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        if let Some((format, text)) = value.split_once(':') {
            match format {
                "sha256" => {
                    if text.is_empty() {
                        Err(anyhow!("Invalid digest string '{value}': no digest"))
                    } else {
                        Ok(Self::Sha256(text.to_owned()))
                    }
                }
                _ => Err(anyhow!(
                    "Invalid digest string '{value}': format must be sha256"
                )),
            }
        } else {
            Err(anyhow!(
                "Invalid digest string '{value}': format must be 'sha256:...'"
            ))
        }
    }
}

impl ComponentDigest {
    fn verify(&self, bytes: &[u8]) -> anyhow::Result<()> {
        match self {
            Self::Sha256(expected) => {
                let actual = &bytes_sha256_string(bytes);
                if expected == actual {
                    Ok(())
                } else {
                    Err(anyhow!("Downloaded file does not match specified digest: expected {expected}, actual {actual}"))
                }
            }
        }
    }
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
