//! Client for distributing Spin applications using OCI registries.

use anyhow::{bail, Context, Result};
use docker_credential::{CredentialRetrievalError, DockerCredential};
use oci_distribution::{
    client::{Config, ImageLayer},
    manifest::OciImageManifest,
    secrets::RegistryAuth,
    Reference,
};
use spin_app::locked::{ContentPath, ContentRef};
use spin_loader::oci::cache::Cache;
use spin_manifest::Application;
use tokio::fs;
use walkdir::WalkDir;

use std::path::{Path, PathBuf};

// TODO: the media types for application, wasm module, and data layer are not final.
const SPIN_APPLICATION_MEDIA_TYPE: &str = "application/vnd.fermyon.spin.application.v1+config";
const WASM_LAYER_MEDIA_TYPE: &str = "application/vnd.wasm.content.layer.v1+wasm";
const DATA_MEDIATYPE: &str = "application/vnd.wasm.content.layer.v1+data";

/// Client for interacting with an OCI registry for Spin applications.
pub struct Client {
    /// Global cache for the metadata, Wasm modules, and static assets pulled from OCI registries.
    pub cache: Cache,
    oci: oci_distribution::Client,
}

impl Client {
    /// Create a new instance of an OCI client for distributing Spin applications.
    pub async fn new(insecure: bool, cache_root: Option<PathBuf>) -> Result<Self> {
        let client = oci_distribution::Client::new(Self::build_config(insecure));
        let cache = Cache::new(cache_root).await?;

        Ok(Self { oci: client, cache })
    }

    /// Push a Spin application to an OCI registry.
    pub async fn push(&mut self, app: &Application, reference: impl AsRef<str>) -> Result<()> {
        let reference: Reference = reference
            .as_ref()
            .parse()
            .with_context(|| format!("cannot parse reference {}", reference.as_ref()))?;
        let auth = Self::auth(&reference)?;
        let working_dir = tempfile::tempdir()?;

        // Create a locked application from the application manifest.
        // TODO: We don't need an extra copy here for each asset to prepare the application.
        // We should be able to use assets::collect instead when constructing the locked app.
        let locked = spin_trigger::locked::build_locked_app(app.clone(), working_dir.path())
            .context("cannot create locked app")?;
        let mut locked = locked.clone();

        // For each component in the application, add layers for the wasm module and
        // all static assets and update the locked application with the file digests.
        let mut layers = Vec::new();
        let mut components = Vec::new();

        for mut c in locked.components {
            // Add the wasm module for the component as layers.
            let source = c
                .clone()
                .source
                .content
                .source
                .context("component loaded from disk should contain a file source")?;

            let source = spin_trigger::parse_file_url(source.as_str())?;
            let layer = Self::wasm_layer(&source).await?;
            let digest = &layer.sha256_digest();
            layers.push(layer);

            // Update the module source with the content digest of the layer.
            c.source.content = ContentRef {
                source: None,
                digest: Some(digest.clone()),
            };

            // Add a layer for each file referenced in the mount directory.
            // Note that this is in fact a directory, and not a single file, so we need to
            // recursively traverse it and add layers for each file.
            let mut files = Vec::new();
            for f in c.files {
                let source = f
                    .content
                    .source
                    .context("file mount loaded from disk should contain a file source")?;
                let source = spin_trigger::parse_file_url(source.as_str())?;
                // Traverse each mount directory, add all static assets as layers, then update the
                // locked application file with the file digest.
                for entry in WalkDir::new(&source) {
                    let entry = entry?;
                    if entry.file_type().is_file() && !entry.file_type().is_dir() {
                        tracing::trace!(
                            "Adding new layer for asset {:?}",
                            spin_loader::to_relative(entry.path(), &source)?
                        );
                        let layer = Self::data_layer(entry.path()).await?;

                        let digest = &layer.sha256_digest();
                        layers.push(layer);

                        files.push(ContentPath {
                            content: ContentRef {
                                source: None,
                                digest: Some(digest.clone()),
                            },
                            path: PathBuf::from(spin_loader::to_relative(entry.path(), &source)?),
                        });
                    }
                }
            }
            c.files = files;
            components.push(c);
        }
        locked.components = components;
        locked.metadata.remove(&"origin".to_string());

        let oci_config = Config {
            data: serde_json::to_vec(&locked)?,
            media_type: SPIN_APPLICATION_MEDIA_TYPE.to_string(),
            annotations: None,
        };
        let manifest = OciImageManifest::build(&layers, &oci_config, None);
        let response = self
            .oci
            .push(&reference, &layers, oci_config, &auth, Some(manifest))
            .await
            .map(|push_response| push_response.manifest_url)
            .context("cannot push Spin application")?;

        tracing::info!("Pushed {:?}", response);

        Ok(())
    }

    /// Pull a Spin application from an OCI registry.
    pub async fn pull(&mut self, reference: &str) -> Result<()> {
        let reference: Reference = reference.parse().context("cannot parse reference")?;
        let auth = Self::auth(&reference)?;

        // Pull the manifest from the registry.
        let (manifest, digest) = self.oci.pull_image_manifest(&reference, &auth).await?;

        let manifest_json = serde_json::to_string(&manifest)?;
        tracing::debug!("Pulled manifest: {}", manifest_json);

        // Write the manifest in `<cache_root>/registry/oci/manifests/repository:<tag_or_latest>/manifest.json`
        let m = self.cache.oci_manifest_path(&reference.to_string()).await?;
        fs::write(&m, &manifest_json).await?;

        let mut cfg_bytes = Vec::new();
        self.oci
            .pull_blob(&reference, &manifest.config.digest, &mut cfg_bytes)
            .await?;
        let cfg = std::str::from_utf8(&cfg_bytes)?;
        tracing::debug!("Pulled config: {}", cfg);

        // Write the config object in `<cache_root>/registry/oci/manifests/repository:<tag_or_latest>/config.json`
        let c = self.cache.lockfile_path(&reference.to_string()).await?;
        fs::write(&c, &cfg).await?;

        // If a layer is a Wasm module, write it in the Wasm directory.
        // Otherwise, write it in the data directory.
        for layer in manifest.layers {
            // Skip pulling if the digest already exists in the wasm or data directories.
            if self.cache.wasm_file(&layer.digest).is_ok()
                || self.cache.data_file(&layer.digest).is_ok()
            {
                tracing::debug!("Layer {} already exists in cache", &layer.digest);
                continue;
            }
            tracing::debug!("Pulling layer {}", &layer.digest);
            let mut bytes = Vec::new();
            self.oci
                .pull_blob(&reference, &layer.digest, &mut bytes)
                .await?;

            match layer.media_type.as_str() {
                WASM_LAYER_MEDIA_TYPE => self.cache.write_wasm(&bytes, &layer.digest).await?,
                _ => self.cache.write_data(&bytes, &layer.digest).await?,
            }
        }

        tracing::info!("Pulled {}@{}", reference, digest);

        Ok(())
    }

    /// Create a new wasm layer based on a file.
    pub async fn wasm_layer(file: &Path) -> Result<ImageLayer> {
        tracing::log::trace!("Reading wasm module from {:?}", file);
        Ok(ImageLayer::new(
            fs::read(file).await.context("cannot read wasm module")?,
            WASM_LAYER_MEDIA_TYPE.to_string(),
            None,
        ))
    }

    /// Create a new data layer based on a file.
    pub async fn data_layer(file: &Path) -> Result<ImageLayer> {
        tracing::log::trace!("Reading data file from {:?}", file);
        Ok(ImageLayer::new(
            fs::read(&file).await?,
            DATA_MEDIATYPE.to_string(),
            None,
        ))
    }

    /// Construct the registry authentication based on the reference.
    fn auth(reference: &Reference) -> Result<RegistryAuth> {
        let server = reference
            .resolve_registry()
            .strip_suffix('/')
            .unwrap_or_else(|| reference.resolve_registry());

        let creds = docker_credential::get_credential(server);
        match creds {
            Err(CredentialRetrievalError::ConfigNotFound) => Ok(RegistryAuth::Anonymous),
            Err(CredentialRetrievalError::NoCredentialConfigured) => Ok(RegistryAuth::Anonymous),
            Err(CredentialRetrievalError::ConfigReadError) => Ok(RegistryAuth::Anonymous),
            Err(e) => bail!("Error handling docker configuration file: {}", e),

            Ok(DockerCredential::UsernamePassword(username, password)) => {
                tracing::trace!("Found docker credentials");
                Ok(RegistryAuth::Basic(username, password))
            }
            Ok(DockerCredential::IdentityToken(_)) => {
                println!("Cannot use contents of docker config, identity token not supported. Using anonymous auth");
                Ok(RegistryAuth::Anonymous)
            }
        }
    }

    /// Build the OCI client configuration given the insecure option.
    fn build_config(insecure: bool) -> oci_distribution::client::ClientConfig {
        let protocol = if insecure {
            oci_distribution::client::ClientProtocol::Http
        } else {
            oci_distribution::client::ClientProtocol::Https
        };

        oci_distribution::client::ClientConfig {
            protocol,
            ..Default::default()
        }
    }
}
