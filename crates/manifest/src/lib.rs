//! Configuration of an application for the Spin runtime.

#![deny(missing_docs)]

use std::{
    collections::HashMap,
    fmt::{Debug, Formatter},
    path::PathBuf,
};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// A trigger error.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Invalid config key.
    #[error("invalid trigger type")]
    InvalidTriggerType,
    /// No 'type' key in trigger declaration.
    #[error("the application did not specify a trigger type")]
    MissingTriggerType,
    /// No 'type' key in trigger declaration.
    #[error("could not load application trigger parameters: {0}")]
    InvalidTriggerTypeParameters(String),
    /// Non-string 'type' key in trigger declaration.
    #[error("the trigger type must be a string")]
    NonStringTriggerType,
}

/// An ordered map of component IDs to some value.
pub type ComponentMap<T> = IndexMap<String, T>;

/// Application configuration.
#[derive(Clone, Debug)]
pub struct Application {
    /// General application information.
    pub info: ApplicationInformation,
    /// Application-specific configuration variables.
    pub variables: HashMap<String, Variable>,
    /// Configuration for the application components.
    pub components: Vec<CoreComponent>,
    /// Configuration for the components' triggers.
    pub component_triggers: ComponentMap<TriggerConfig>,
}

/// Spin API version.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SpinVersion {
    /// Version 1 format.
    V1,
}

/// General application information.
#[derive(Clone, Debug)]
pub struct ApplicationInformation {
    /// Spin API version.
    pub spin_version: SpinVersion,
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
    /// Description of the component.
    pub description: Option<String>,
    /// Per-component WebAssembly configuration.
    pub wasm: WasmConfig,
    /// Per-component configuration values.
    pub config: HashMap<String, String>,
}

/// A custom config variable.
#[derive(Clone, Debug)]
pub struct Variable {
    /// If set, the default value of this variable. If unset, this variable is required.
    pub default: Option<String>,
    /// If set, this variable's value should be treated as sensitive (e.g. not logged).
    pub secret: bool,
}

/// The location from which an application was loaded.
#[derive(Clone, Debug, PartialEq, Eq)]
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
#[serde(
    deny_unknown_fields,
    rename_all = "camelCase",
    try_from = "ApplicationTriggerDeserialised",
    into = "ApplicationTriggerSerialised"
)]
pub enum ApplicationTrigger {
    /// HTTP trigger type.
    Http(HttpTriggerConfiguration),
    /// Redis trigger type.
    Redis(RedisTriggerConfiguration),
    /// A trigger type that is not built in.
    External(ExternalTriggerConfiguration),
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase", untagged)]
enum ApplicationTriggerSerialised {
    Internal(InternalApplicationTriggerSerialised),
    /// A trigger type that is not built in.
    External(HashMap<String, toml::Value>),
}

/// Deserialisation helper - we need all unmatched `trigger.type` values to
/// map to `ApplicationTrigger::External`, but `#[serde(other)]` can
/// only be applied to unit types.  The following types cause recognised
/// tags to map to the Internal case and unrecognised ones to the
/// External case.
#[derive(Deserialize)]
struct ApplicationTriggerDeserialised {
    #[serde(rename = "type")]
    trigger_type: String,
    #[serde(flatten)]
    parameters: toml::Value,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase", tag = "type")]
enum InternalApplicationTriggerSerialised {
    /// HTTP trigger type.
    Http(HttpTriggerConfiguration),
    /// Redis trigger type.
    Redis(RedisTriggerConfiguration),
}

impl TryFrom<ApplicationTriggerDeserialised> for ApplicationTrigger {
    type Error = Error;

    fn try_from(value: ApplicationTriggerDeserialised) -> Result<Self, Self::Error> {
        let trigger = match value.trigger_type.as_str() {
            "http" => ApplicationTrigger::Http(
                HttpTriggerConfiguration::deserialize(value.parameters)
                    .map_err(|e| Error::InvalidTriggerTypeParameters(e.to_string()))?,
            ),
            "redis" => ApplicationTrigger::Redis(
                RedisTriggerConfiguration::deserialize(value.parameters)
                    .map_err(|e| Error::InvalidTriggerTypeParameters(e.to_string()))?,
            ),
            _ => ApplicationTrigger::External(ExternalTriggerConfiguration {
                trigger_type: value.trigger_type,
                parameters: HashMap::deserialize(value.parameters)
                    .map_err(|e| Error::InvalidTriggerTypeParameters(e.to_string()))?,
            }),
        };
        Ok(trigger)
    }
}

impl From<ApplicationTrigger> for ApplicationTriggerSerialised {
    fn from(value: ApplicationTrigger) -> Self {
        match value {
            ApplicationTrigger::Http(h) => {
                Self::Internal(InternalApplicationTriggerSerialised::Http(h))
            }
            ApplicationTrigger::Redis(r) => {
                Self::Internal(InternalApplicationTriggerSerialised::Redis(r))
            }
            ApplicationTrigger::External(e) => {
                let ty = e.trigger_type;
                let mut map = e.parameters;
                map.insert("type".to_owned(), toml::Value::String(ty));
                Self::External(map)
            }
        }
    }
}

/// HTTP trigger configuration.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct HttpTriggerConfiguration {
    /// Base path for the HTTP application.
    pub base: String,
}

impl Default for HttpTriggerConfiguration {
    fn default() -> Self {
        Self { base: "/".into() }
    }
}

impl TryFrom<ApplicationTrigger> for HttpTriggerConfiguration {
    type Error = Error;

    fn try_from(trigger: ApplicationTrigger) -> Result<Self, Self::Error> {
        match trigger {
            ApplicationTrigger::Http(http) => Ok(http),
            _ => Err(Error::InvalidTriggerType),
        }
    }
}

/// Redis trigger configuration.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct RedisTriggerConfiguration {
    /// Address of Redis server.
    pub address: String,
}

impl TryFrom<ApplicationTrigger> for RedisTriggerConfiguration {
    type Error = Error;

    fn try_from(trigger: ApplicationTrigger) -> Result<Self, Self::Error> {
        match trigger {
            ApplicationTrigger::Redis(redis) => Ok(redis),
            _ => Err(Error::InvalidTriggerType),
        }
    }
}

/// External trigger configuration
#[derive(Clone, Debug, PartialEq)]
pub struct ExternalTriggerConfiguration {
    trigger_type: String,
    parameters: HashMap<String, toml::Value>,
}

impl ExternalTriggerConfiguration {
    /// The trigger type.
    pub fn trigger_type(&self) -> &str {
        &self.trigger_type
    }
}

/// WebAssembly configuration.
#[derive(Clone, Debug, Default)]
pub struct WasmConfig {
    /// Environment variables to be mapped inside the Wasm module at runtime.
    pub environment: HashMap<String, String>,
    /// List of directory mounts that need to be mapped inside the WebAssembly module.
    pub mounts: Vec<DirectoryMount>,
    /// Optional list of HTTP hosts the component is allowed to connect.
    pub allowed_http_hosts: Vec<String>,
    /// Optional list of key-value stores the component is allowed to use.
    pub key_value_stores: Vec<String>,
    /// Optional list of sqlite databases the component is allowed to use.
    pub sqlite_databases: Vec<String>,
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
#[derive(Clone)]
pub enum ModuleSource {
    /// A local path to the entrypoint Wasm module.
    FileReference(PathBuf),

    /// A buffer that contains the entrypoint Wasm module and
    /// source information.
    Buffer(Vec<u8>, String),
}

impl Debug for ModuleSource {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        match self {
            ModuleSource::FileReference(fp) => {
                f.debug_struct("FileReference").field("file", fp).finish()
            }
            ModuleSource::Buffer(bytes, info) => f
                .debug_struct("Buffer")
                .field("len", &bytes.len())
                .field("info", info)
                .finish(),
        }
    }
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

/// The executor for the HTTP component.
/// The component can either implement the Spin HTTP interface,
/// or the Wagi CGI interface.
///
/// If an executor is not specified, the inferred default is `HttpExecutor::Spin`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase", tag = "type")]
pub enum HttpExecutor {
    /// The component implements the Spin HTTP interface.
    Spin,
    /// The component implements the Wagi CGI interface.
    Wagi(WagiConfig),
}

impl Default for HttpExecutor {
    fn default() -> Self {
        Self::Spin
    }
}

/// Wagi specific configuration for the http executor.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct WagiConfig {
    /// The name of the entrypoint.
    pub entrypoint: String,
    /// A string representation of the argv array.
    ///
    /// This should be a space-separate list of strings. The value
    /// ${SCRIPT_NAME} will be replaced with the Wagi SCRIPT_NAME,
    /// and the value ${ARGS} will be replaced with the query parameter
    /// name/value pairs presented as args. For example,
    /// `param1=val1&param2=val2` will become `param1=val1 param2=val2`,
    /// which will then be presented to the program as two arguments
    /// in argv.
    pub argv: String,
}

impl Default for WagiConfig {
    fn default() -> WagiConfig {
        /// This is the default Wagi entrypoint.
        const WAGI_DEFAULT_ENTRYPOINT: &str = "_start";
        const WAGI_DEFAULT_ARGV: &str = "${SCRIPT_NAME} ${ARGS}";

        WagiConfig {
            entrypoint: WAGI_DEFAULT_ENTRYPOINT.to_owned(),
            argv: WAGI_DEFAULT_ARGV.to_owned(),
        }
    }
}

/// Configuration for the Redis trigger.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RedisConfig {
    /// Redis channel to subscribe.
    pub channel: String,
    /// The Redis executor the component requires.
    pub executor: Option<RedisExecutor>,
}

/// The executor for the Redis component.
///
/// If an executor is not specified, the inferred default is `RedisExecutor::Spin`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase", tag = "type")]
pub enum RedisExecutor {
    /// The component implements the Spin Redis interface.
    Spin,
}

impl Default for RedisExecutor {
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
    /// Redis trigger configuration
    Redis(RedisConfig),
    /// External trigger configuration
    External(HashMap<String, toml::Value>),
}

impl Default for TriggerConfig {
    fn default() -> Self {
        Self::Http(Default::default())
    }
}

impl TryFrom<TriggerConfig> for HttpConfig {
    type Error = Error;

    fn try_from(trigger: TriggerConfig) -> Result<Self, Self::Error> {
        match trigger {
            TriggerConfig::Http(http) => Ok(http),
            _ => Err(Error::InvalidTriggerType),
        }
    }
}

impl TryFrom<TriggerConfig> for RedisConfig {
    type Error = Error;

    fn try_from(trigger: TriggerConfig) -> Result<Self, Self::Error> {
        match trigger {
            TriggerConfig::Redis(redis) => Ok(redis),
            _ => Err(Error::InvalidTriggerType),
        }
    }
}
