use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
};

/// Application configuration file format.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AppManifest {
    pub trigger: crate::ApplicationTrigger,

    /// Configuration for the application components.
    #[serde(rename = "component")]
    pub components: Vec<ComponentManifest>,
}

/// Core component configuration.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ComponentManifest {
    /// The module source.
    pub source: String,
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
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RawWasmConfig {
    /// Environment variables to be mapped inside the Wasm module at runtime.
    pub environment: Option<HashMap<String, String>>,
    /// The parcel group to be mapped inside the Wasm module at runtime.
    pub files: Option<String>,
    /// Optional list of HTTP hosts the component is allowed to connect.
    pub allowed_http_hosts: Option<Vec<String>>,
}
