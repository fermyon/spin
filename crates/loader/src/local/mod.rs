//! Functionality to get a prepared Spin application configuration from spin.toml.

#![deny(missing_docs)]

/// Module to prepare the assets for the components of an application.
pub mod assets;
/// Configuration representation for a Spin application as a local spin.toml file.
pub mod config;

#[cfg(test)]
mod tests;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Context, Result};
use futures::future;
use itertools::Itertools;
use outbound_http::allowed_http_hosts::validate_allowed_http_hosts;
use path_absolutize::Absolutize;
use reqwest::Url;
use spin_manifest::{
    Application, ApplicationInformation, ApplicationOrigin, ApplicationTrigger, CoreComponent,
    HttpConfig, ModuleSource, RedisConfig, SpinVersion, TriggerConfig, WasmConfig,
};
use tokio::{fs::File, io::AsyncReadExt};

use crate::{
    cache::Cache, digest::bytes_sha256_string, local::config::is_missing_tag_error,
    validation::validate_key_value_stores,
};
use config::{
    FileComponentUrlSource, RawAppInformation, RawAppManifest, RawAppManifestAnyVersion,
    RawComponentManifest,
};

use self::config::VersionTagLoader;

/// Given the path to a spin.toml manifest file, prepare its assets locally and
/// get a prepared application configuration consumable by a Spin execution context.
/// If a directory is provided, use it as the base directory to expand the assets,
/// otherwise create a new temporary directory.
pub async fn from_file(
    app: impl AsRef<Path>,
    base_dst: Option<impl AsRef<Path>>,
) -> Result<Application> {
    let app = absolutize(app)?;
    let manifest = raw_manifest_from_file(&app).await?;
    validate_raw_app_manifest(&manifest)?;

    prepare_any_version(manifest, app, base_dst).await
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

    let manifest: RawAppManifestAnyVersion = raw_manifest_from_slice(&buf)
        .with_context(|| anyhow!("Cannot read manifest file from {:?}", app.as_ref()))?;

    Ok(manifest)
}

fn raw_manifest_from_slice(buf: &[u8]) -> Result<RawAppManifestAnyVersion> {
    use serde::Deserialize;
    let tl = toml::from_slice(buf);
    let tl = if is_missing_tag_error(&tl) {
        tl.context("Manifest must contain spin_manifest_version with a value of \"1\"")?
    } else {
        tl?
    };

    match tl {
        VersionTagLoader::OldV1 { rest, .. } => {
            // let rest_text = toml::to_string_pretty(&rest)?;
            // println!("{rest_text}");
            let raw = RawAppManifest::deserialize(rest)?;
            Ok(RawAppManifestAnyVersion::V1(raw))
        }
        VersionTagLoader::NewV1 { rest, .. } => {
            let raw = RawAppManifest::deserialize(rest)?;
            Ok(RawAppManifestAnyVersion::V1(raw))
        }
    }
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
    base_dst: Option<impl AsRef<Path>>,
) -> Result<Application> {
    let manifest = raw.into_v1();
    prepare(manifest, src, base_dst).await
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
    let manifest = raw.as_v1();
    manifest
        .components
        .iter()
        .try_for_each(|c| validate_allowed_http_hosts(&c.wasm.allowed_http_hosts))?;
    manifest
        .components
        .iter()
        .try_for_each(|c| validate_key_value_stores(&c.wasm.key_value_stores))?;

    Ok(())
}

/// Converts a raw application manifest into Spin configuration.
async fn prepare(
    raw: RawAppManifest,
    src: impl AsRef<Path>,
    base_dst: Option<impl AsRef<Path>>,
) -> Result<Application> {
    let info = info(raw.info, &src);

    error_on_duplicate_ids(raw.components.clone())?;

    let component_triggers = raw
        .components
        .iter()
        .map(|c| {
            resolve_trigger(&info.trigger, c.trigger.clone())
                .map(|t| (c.id.clone(), t))
                .map_err(|e| anyhow!("{e} in component trigger '{}'", c.id))
        })
        .collect::<Result<_, _>>()?;

    let components = future::join_all(
        raw.components
            .into_iter()
            .map(|c| async { core(c, &src, base_dst.as_ref()).await })
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
    base_dst: Option<impl AsRef<Path>>,
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
            assets::prepare_component(&f, src, base_dst, &id, &exclude_files).await?
        }
        None => vec![],
    };
    let environment = raw.wasm.environment.unwrap_or_default();
    let allowed_http_hosts = raw.wasm.allowed_http_hosts.unwrap_or_default();
    let key_value_stores = raw.wasm.key_value_stores.unwrap_or_default();
    let wasm = WasmConfig {
        environment,
        mounts,
        allowed_http_hosts,
        key_value_stores,
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
        // TODO: when `spin up` integrates running an app from OCI, pass the configured
        // cache root to this function. For now, use the default cache directory.
        let cache = Cache::new(None).await?;
        match cache.wasm_file(self.digest_str()) {
            Ok(p) => {
                tracing::debug!(
                    "Using local cache for module source {} with digest {}",
                    &self.url,
                    &self.digest_str()
                );
                Ok(tokio::fs::read(p).await?)
            }
            Err(_) => {
                tracing::debug!("Pulling module from URL {}", &self.url);
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
                cache.write_wasm(&bytes, self.digest_str()).await?;

                Ok(bytes)
            }
        }
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

/// Resolves the raw map of a component trigger settings into a
/// typed component trigger config object.
pub fn resolve_trigger(
    app_trigger: &ApplicationTrigger,
    partial: toml::Value,
) -> Result<TriggerConfig> {
    use serde::Deserialize;
    let tc = match app_trigger {
        ApplicationTrigger::Http(_) => TriggerConfig::Http(HttpConfig::deserialize(partial)?),
        ApplicationTrigger::Redis(_) => TriggerConfig::Redis(RedisConfig::deserialize(partial)?),
        ApplicationTrigger::External(_) => TriggerConfig::External(HashMap::deserialize(partial)?),
    };
    Ok(tc)
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
        origin: ApplicationOrigin::File(src.as_ref().to_path_buf()),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn load_test_manifest(app_trigger: &str, comp_trigger: &str) -> RawAppManifestAnyVersion {
        let manifest_toml = format!(
            r#"
spin_version = "1"
name = "test"
trigger = {app_trigger}
version = "0.0.1"

[[component]]
id = "test"
source = "nonexistent.wasm"
[component.trigger]
{comp_trigger}
"#
        );

        let manifest = raw_manifest_from_slice(manifest_toml.as_bytes()).unwrap();
        validate_raw_app_manifest(&manifest).unwrap();

        manifest
    }

    fn try_load_generic_manifest(
        app_insert: &str,
        comp_insert: &str,
    ) -> anyhow::Result<RawAppManifestAnyVersion> {
        let manifest_toml = format!(
            r#"
spin_manifest_version = "1"
name = "test"
version = "0.0.1"
{app_insert}

[[component]]
id = "test"
{comp_insert}
"#
        );

        raw_manifest_from_slice(manifest_toml.as_bytes())
    }

    #[test]
    fn can_parse_http_trigger() {
        let m = load_test_manifest(r#"{ type = "http", base = "/" }"#, r#"route = "/...""#);
        let m1 = m.into_v1();
        let t = &m1.info.trigger;
        let ct = resolve_trigger(t, m1.components[0].trigger.clone())
            .expect("Should have resolved trigger");
        assert!(matches!(t, ApplicationTrigger::Http(_)));
        assert!(matches!(ct, TriggerConfig::Http(_)));
    }

    #[test]
    fn can_parse_redis_trigger() {
        let m = load_test_manifest(
            r#"{ type = "redis", address = "dummy" }"#,
            r#"channel = "chan""#,
        );

        let m1 = m.into_v1();
        let t = m1.info.trigger;
        let ct = resolve_trigger(&t, m1.components[0].trigger.clone())
            .expect("Should have resolved trigger");
        assert!(matches!(t, ApplicationTrigger::Redis(_)));
        assert!(matches!(ct, TriggerConfig::Redis(_)));
    }

    #[test]
    fn can_parse_unknown_trigger() {
        let m = load_test_manifest(r#"{ type = "pounce" }"#, r#"on = "MY KNEES""#);

        let m1 = m.into_v1();
        let t = m1.info.trigger;
        let ct = resolve_trigger(&t, m1.components[0].trigger.clone())
            .expect("Should have resolved trigger");
        assert!(matches!(t, ApplicationTrigger::External(_)));
        assert!(matches!(ct, TriggerConfig::External(_)));
    }

    #[test]
    fn external_triggers_can_have_same_config_keys_as_builtins() {
        let m = load_test_manifest(
            r#"{ type = "pounce" }"#,
            r#"route = "over the cat tree and out of the sun""#,
        );

        let m1 = m.into_v1();
        let t = m1.info.trigger;
        let ct = resolve_trigger(&t, m1.components[0].trigger.clone())
            .expect("Should have resolved trigger");
        assert!(matches!(t, ApplicationTrigger::External(_)));
        assert!(matches!(ct, TriggerConfig::External(_)));
    }

    #[test]
    fn bad_app_trigger_gives_good_error() {
        let e = try_load_generic_manifest(
            r#"trigger = { type = "http", bass = "all about that" }"#,
            r#"source = "nonexistent.wasm"
            [component.trigger]
            route = "/...""#,
        )
        .expect_err("Should not, in fact, have been all about that bass");

        assert!(
            e.to_string()
                .contains("incorrect or missing required settings for trigger type"),
            "{e}"
        );
    }

    #[test]
    fn bad_app_field_gives_good_error() {
        let e = try_load_generic_manifest(
            r#"trigger = { type = "http", base = "/" }
            deZZZcription = "so descriptive""#,
            r#"source = "nonexistent.wasm"
            [component.trigger]
            route = "/...""#,
        )
        .expect_err("expected bad app field to not load");

        assert!(
            e.to_string().contains("unknown field `deZZZcription`"),
            "{e}"
        );
    }

    #[tokio::test]
    async fn bad_component_trigger_gives_good_error() {
        // Component trigger errors aren't detected until the
        // 'prepare' stage of the load sequence.
        let m = try_load_generic_manifest(
            r#"trigger = { type = "http", base = "/" }"#,
            r#"source = "nonexistent.wasm"
            [component.trigger]
            root = "/...""#,
        )
        .unwrap();
        let e = prepare_any_version(m, ".", None::<PathBuf>)
            .await
            .expect_err("expected bad component trigger to not load");

        assert!(
            e.to_string()
                .contains("missing field `route` in component trigger 'test'"),
            "{e}"
        );
    }

    #[test]
    fn bad_component_field_gives_good_error() {
        let e = try_load_generic_manifest(
            r#"trigger = { type = "http", base = "/" }"#,
            r#"sauce = "nonexistent.wasm"
            [component.trigger]
            route = "/...""#,
        )
        .expect_err("expected bad component field to not load");

        assert!(e.to_string().contains("missing field `source`"), "{e}");
    }
}
