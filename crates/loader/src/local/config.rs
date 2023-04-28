//! Internal configuration for converting a local spin.toml application manifest,
//! WebAssembly modules, and static assets into a configuration runnable by the
//! Spin execution context.

#![deny(missing_docs)]

use serde::{Deserialize, Serialize};
use spin_manifest::{ApplicationTrigger, TriggerConfig};
use std::{collections::HashMap, path::PathBuf};

use crate::common::RawVariable;

/// Container for any version of the manifest.
pub type RawAppManifestAnyVersion = RawAppManifestAnyVersionImpl<TriggerConfig>;
/// Application configuration local file format.
/// This is the main structure spin.toml deserializes into.
pub type RawAppManifest = RawAppManifestImpl<TriggerConfig>;
/// Core component configuration.
pub type RawComponentManifest = RawComponentManifestImpl<TriggerConfig>;

pub(crate) type RawAppManifestAnyVersionPartial = RawAppManifestAnyVersionImpl<toml::Value>;
pub(crate) type RawComponentManifestPartial = RawComponentManifestImpl<toml::Value>;

/// Container for any version of the manifest.
#[derive(Clone, Debug, Deserialize)]
pub struct RawAppManifestAnyVersionImpl<C> {
    #[serde(alias = "spin_version")]
    //We don't actually use the version yet
    #[allow(dead_code)]
    /// Version key name
    spin_manifest_version: FixedStringVersion<1>,
    /// Manifest
    #[serde(flatten)]
    manifest: RawAppManifestImpl<C>,
}

impl<C> RawAppManifestAnyVersionImpl<C> {
    /// Creates a `RawAppManifestAnyVersionImpl` from `RawAppManifestImpl`
    pub fn from_manifest(manifest: RawAppManifestImpl<C>) -> Self {
        Self {
            manifest,
            spin_manifest_version: FixedStringVersion::default(),
        }
    }
    /// Converts `RawAppManifestAnyVersionImpl` into underlying V1 manifest
    pub fn into_v1(self) -> RawAppManifestImpl<C> {
        self.manifest
    }

    /// Returns a reference to the underlying V1 manifest
    pub fn as_v1(&self) -> &RawAppManifestImpl<C> {
        &self.manifest
    }
}

/// Application configuration local file format.
/// This is the main structure spin.toml deserializes into.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct RawAppManifestImpl<C> {
    /// General application information.
    #[serde(flatten)]
    pub info: RawAppInformation,

    /// Configuration for the application components.
    #[serde(rename = "component")]
    pub components: Vec<RawComponentManifestImpl<C>>,

    /// Application-specific configuration schema.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub variables: HashMap<String, RawVariable>,
}

/// General application information.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct RawAppInformation {
    /// Name of the application.
    pub name: String,
    /// Version of the application.
    pub version: String,
    /// Description of the application.
    pub description: Option<String>,
    /// Authors of the application.
    pub authors: Option<Vec<String>>,
    /// Trigger for the application.
    pub trigger: ApplicationTrigger,
    /// Namespace for the application. (deprecated)
    pub namespace: Option<String>,
}

/// Core component configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct RawComponentManifestImpl<C> {
    /// The module source.
    pub source: RawModuleSource,
    /// ID of the component. Used at runtime to select between
    /// multiple components of the same application.
    pub id: String,
    /// Description of the component.
    pub description: Option<String>,
    /// Per-component WebAssembly configuration.
    #[serde(flatten)]
    pub wasm: RawWasmConfig,
    /// Trigger configuration.
    pub trigger: C,
    /// Build configuration for the component.
    pub build: Option<RawBuildConfig>,
    /// Component-specific configuration values.
    pub config: Option<HashMap<String, String>>,
}

/// Build configuration for the component.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct RawBuildConfig {
    /// Build command.
    pub command: String,
    /// Working directory in which the build command is executed. It must be
    /// relative to the directory in which `spin.toml` is located.
    pub workdir: Option<PathBuf>,
    /// List of glob patterns to watch for changes. Used by spin watch to
    /// re-execute spin build and spin up when your source changes.
    pub watch: Option<Vec<String>>,
}

/// WebAssembly configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct RawWasmConfig {
    /// Files to be mapped inside the Wasm module at runtime.
    ///
    /// In the local configuration file, this is a vector, each element of which
    /// is either a file path or glob relative to the spin.toml file, or a
    /// mapping of a source path to an absolute mount path in the guest.
    pub files: Option<Vec<RawFileMount>>,
    /// Optional list of file path or glob relative to the spin.toml that don't mount to wasm.
    /// When exclude_files conflict with files config, exclude_files take precedence.
    pub exclude_files: Option<Vec<String>>,
    /// Optional list of HTTP hosts the component is allowed to connect.
    pub allowed_http_hosts: Option<Vec<String>>,
    /// Optional list of key-value stores the component is allowed to use.
    pub key_value_stores: Option<Vec<String>>,
    /// Environment variables to be mapped inside the Wasm module at runtime.
    pub environment: Option<HashMap<String, String>>,
}

/// An entry in the `files` list mapping a source path to an absolute
/// mount path in the guest.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct RawDirectoryPlacement {
    /// The source to mount.
    pub source: PathBuf,
    /// Where to mount the directory specified in `source`.
    pub destination: PathBuf,
}

/// A specification for a file or set of files to mount in the
/// Wasm module.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case", untagged)]
pub enum RawFileMount {
    /// Mount a specified directory at a specified location.
    Placement(RawDirectoryPlacement),
    /// Mount a file or set of files at their relative path.
    Pattern(String),
}

/// Source for the module.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case", untagged)]
pub enum RawModuleSource {
    /// Local path or parcel reference to a module that needs to be linked.
    FileReference(PathBuf),
    /// Reference to a Wasm file at a URL
    Url(FileComponentUrlSource),
}

/// A component source from Bindle.
/// TODO
/// The component and its entrypoint should be pulled from Bindle.
/// This assumes access to the Bindle server.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct FileComponentBindleSource {
    /// Reference to the bindle (name/version)
    pub reference: String,
    /// Parcel to use from the bindle.
    pub parcel: String,
}
/// A component source from a URL.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct FileComponentUrlSource {
    /// The URL of the Wasm binary.
    pub url: String,
    /// The digest of the Wasm binary, used for integrity checking. This must be a
    /// SHA256 digest, in the form `sha256:...`
    pub digest: String,
}

/// FixedStringVersion represents a schema version field with a const value.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub struct FixedStringVersion<const V: usize>;

impl<const V: usize> From<FixedStringVersion<V>> for String {
    fn from(_: FixedStringVersion<V>) -> String {
        V.to_string()
    }
}

impl<const V: usize> TryFrom<String> for FixedStringVersion<V> {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let value: usize = value
            .parse()
            .map_err(|err| format!("invalid version: {}", err))?;
        if value != V {
            return Err(format!("invalid version {} != {}", value, V));
        }
        Ok(Self)
    }
}
