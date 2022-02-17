//! Configuration of an application for the Spin runtime.

#![deny(missing_docs)]

use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

/// Application configuration.
#[derive(Clone, Debug)]
pub struct Configuration<T> {
    /// General application information.
    pub info: ApplicationInformation,
    /// Configuration for the application components.
    pub components: Vec<T>,
}

/// General application information.
#[derive(Clone, Debug)]
pub struct ApplicationInformation {
    /// Spin API version.
    pub api_version: String,
    /// Name of the application.
    pub name: String,
    /// Version of the application.
    pub version: String,
    /// Description of the application.
    pub description: Option<String>,
    /// Authors of the application.
    pub authors: Vec<String>,
    /// Trigger for the application.
    /// Currently, all components of a given application must be
    /// invoked as a result of the same trigger "type".
    /// In the future, applications with mixed triggers might be allowed,
    /// but for now, a component with a different trigger must be part of
    /// a separate application.
    pub trigger: ApplicationTrigger,
    /// Namespace for groupping applications.
    pub namespace: Option<String>,
    /// The location from which the application is loaded.
    pub origin: ApplicationOrigin,
}

/// Core component configuration.
#[derive(Clone, Debug)]
pub struct CoreComponent {
    /// The module source.
    pub source: ModuleSource,
    /// ID of the component. Used at runtime to select between
    /// multiple components of the same application.
    pub id: String,
    /// Per-component WebAssembly configuration.
    /// This takes precedence over the application-level
    /// WebAssembly configuration.
    pub wasm: WasmConfig,
    /// Trigger configuration.
    pub trigger: TriggerConfig,
}

/// The location from which an application was loaded.
#[derive(Clone, Debug, PartialEq)]
pub enum ApplicationOrigin {
    /// The application was loaded from the specified file.
    File(PathBuf),
    /// The application was loaded from the specified bindle.
    Bindle {
        /// Bindle ID for the component.
        id: String,
        /// Bindle server URL.
        server: String,
    },
}

/// The trigger type.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase", tag = "type")]
pub enum ApplicationTrigger {
    /// HTTP trigger type.
    Http(HttpTriggerConfiguration),
}

/// HTTP trigger configuration.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct HttpTriggerConfiguration {
    /// Base path for the HTTP application.
    pub base: String,
}
impl Default for HttpTriggerConfiguration {
    fn default() -> Self {
        Self { base: "/".into() }
    }
}

/// WebAssembly configuration.
#[derive(Clone, Debug)]
pub struct WasmConfig {
    /// Environment variables to be mapped inside the Wasm module at runtime.
    pub environment: HashMap<String, String>,
    /// List of directory mounts that need to be mapped inside the WebAssembly module.
    pub mounts: Vec<DirectoryMount>,
    /// Optional list of HTTP hosts the component is allowed to connect.
    pub allowed_http_hosts: Vec<String>,
}

/// Directory mount for the assets of a component.
#[derive(Clone, Debug)]
pub struct DirectoryMount {
    /// Guest directory destination for mounting inside the module.
    pub guest: String,
    /// Host directory source for mounting inside the module.
    pub host: PathBuf,
}

/// Source for the entrypoint Wasm module of a component.
#[derive(Clone, Debug)]
pub enum ModuleSource {
    /// A local path to the entrypoint Wasm module.
    FileReference(PathBuf),
}

/// Configuration for the HTTP trigger.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HttpConfig {
    /// HTTP route the component will be invoked for.
    pub route: String,
    /// The HTTP executor the component requires.
    pub executor: Option<HttpExecutor>,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            route: "/".to_string(),
            executor: Default::default(),
        }
    }
}

/// The type of interface the component implements.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub enum HttpExecutor {
    /// The component implements the Spin HTTP interface.
    Spin,
    /// The component implements the Wagi interface.
    Wagi,
}

impl Default for HttpExecutor {
    fn default() -> Self {
        Self::Spin
    }
}

/// Trigger configuration.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase", untagged)]
pub enum TriggerConfig {
    /// HTTP trigger configuration
    Http(HttpConfig),
}

impl Default for TriggerConfig {
    fn default() -> Self {
        Self::Http(Default::default())
    }
}
