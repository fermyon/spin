use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::PathBuf,
};

/// Application configuration file format.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AppManifest {
    /// General application information.
    #[serde(flatten)]
    pub info: AppInformation,

    /// Configuration for the application components.
    #[serde(rename = "component")]
    pub components: Vec<ComponentManifest>,
}

/// General application information.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AppInformation {
    /// Name of the application.
    pub name: String,
    /// Version of the application.
    pub version: String,
    /// Description of the application.
    pub description: Option<String>,
    /// Authors of the application.
    pub authors: Option<Vec<String>>,
    /// Trigger for the application.
    ///
    /// Currently, all components of a given application must be
    /// invoked as a result of the same trigger "type".
    /// In the future, applications with mixed triggers might be allowed,
    /// but for now, a component with a different trigger must be part of
    /// a separate application.
    pub trigger: crate::ApplicationTrigger,
    /// TODO
    pub namespace: Option<String>,
}

/// Core component configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ComponentManifest {
    /// The module source.
    pub source: RawModuleSource,
    /// ID of the component. Used at runtime to select between
    /// multiple components of the same application.
    pub id: String,
    /// Per-component WebAssembly configuration.
    /// This takes precedence over the application-level
    /// WebAssembly configuration.
    #[serde(flatten)]
    pub wasm: RawWasmConfig,
    /// Trigger configuration.
    pub trigger: crate::TriggerConfig,
}

/// WebAssembly configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RawWasmConfig {
    /// Environment variables to be mapped inside the Wasm module at runtime.
    pub environment: Option<HashMap<String, String>>,
    /// Files to be mapped inside the Wasm module at runtime.
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
/// This assumes access to the Bindle server.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct FileComponentBindleSource {
    /// Reference to the bindle (name/version)
    pub reference: String,
    /// Parcel to use from the bindle.
    pub parcel: String,
}
