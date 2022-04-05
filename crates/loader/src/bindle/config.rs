use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Application configuration file format.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RawAppManifest {
    /// The application trigger.
    pub trigger: spin_manifest::ApplicationTrigger,

    /// Configuration for the application components.
    #[serde(rename = "component")]
    pub components: Vec<RawComponentManifest>,
}

/// Core component configuration.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RawComponentManifest {
    /// The module source.
    pub source: String,
    /// ID of the component. Used at runtime to select between
    /// multiple components of the same application.
    pub id: String,
    /// Per-component WebAssembly configuration.
    #[serde(flatten)]
    pub wasm: RawWasmConfig,
    /// Trigger configuration.
    pub trigger: spin_manifest::TriggerConfig,
}

/// WebAssembly configuration.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RawWasmConfig {
    /// The parcel group to be mapped inside the Wasm module at runtime.
    pub files: Option<String>,
    /// Optional list of HTTP hosts the component is allowed to connect.
    pub allowed_http_hosts: Option<Vec<String>>,
    /// Environment variables to be mapped inside the Wasm module at runtime.
    pub environment: Option<HashMap<String, String>>,
}
