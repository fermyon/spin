//! Internal configuration for converting a local spin.toml application manifest,
//! WebAssembly modules, and static assets into a configuration runnable by the
//! Spin execution context.

#![deny(missing_docs)]

use serde::{Deserialize, Serialize};
use spin_config::{ApplicationTrigger, TriggerConfig};
use std::{collections::HashMap, path::PathBuf};

/// Application configuration local file format.
/// This is the main structure spin.toml deserializes into.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RawAppManifest {
    /// General application information.
    #[serde(flatten)]
    pub info: RawAppInformation,

    /// Configuration for the application components.
    #[serde(rename = "component")]
    pub components: Vec<RawComponentManifest>,
}

/// General application information.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RawAppInformation {
    /// Spin API version.
    pub api_version: String,
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
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RawComponentManifest {
    /// The module source.
    pub source: RawModuleSource,
    /// ID of the component. Used at runtime to select between
    /// multiple components of the same application.
    pub id: String,
    /// Per-component WebAssembly configuration.
    #[serde(flatten)]
    pub wasm: RawWasmConfig,
    /// Trigger configuration.
    pub trigger: TriggerConfig,
}

/// WebAssembly configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RawWasmConfig {
    /// Environment variables to be mapped inside the Wasm module at runtime.
    pub environment: Option<HashMap<String, String>>,
    /// Files to be mapped inside the Wasm module at runtime.
    ///
    /// In the local configuration file, this is a vector or file paths or
    /// globs relative to the spin.toml file.
    pub files: Option<Vec<String>>,
    /// Optional list of HTTP hosts the component is allowed to connect.
    pub allowed_http_hosts: Option<Vec<String>>,
}

/// Source for the module.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase", untagged)]
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
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct FileComponentBindleSource {
    /// Reference to the bindle (name/version)
    pub reference: String,
    /// Parcel to use from the bindle.
    pub parcel: String,
}
