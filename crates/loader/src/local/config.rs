//! Internal configuration for converting a local spin.toml application manifest,
//! WebAssembly modules, and static assets into a configuration runnable by the
//! Spin execution context.

#![deny(missing_docs)]

use serde::{Deserialize, Serialize};
use spin_manifest::{ApplicationTrigger, TriggerConfig};
use std::{collections::HashMap, path::PathBuf};

use crate::common::RawVariable;

/// Container for any version of the manifest.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "spin_version")]
pub enum RawAppManifestAnyVersion {
    /// A manifest with API version 1.
    #[serde(rename = "1")]
    V1(RawAppManifest),
}

/// Application configuration local file format.
/// This is the main structure spin.toml deserializes into.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct RawAppManifest {
    /// General application information.
    #[serde(flatten)]
    pub info: RawAppInformation,

    /// Configuration for the application components.
    #[serde(rename = "component")]
    pub components: Vec<RawComponentManifest>,

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
    /// Namespace for the application.
    pub namespace: Option<String>,
}

/// Core component configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct RawComponentManifest {
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
    pub trigger: TriggerConfig,
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
    /// Reference to a remote bindle
    Bindle(FileComponentBindleSource),
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
