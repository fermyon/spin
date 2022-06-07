use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};
use serde_value::Value;
use serde_with::{rust::maps_duplicate_key_is_error, skip_serializing_none};

use crate::ManifestVersion;

/// An error in the manifest file.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid component ID: {0}")]
    InvalidComponentId(String),
    #[error("invalid trigger type: {0}")]
    InvalidTriggerType(String),
}

/// Manifest represents a (de)serializable "V1" application manifest.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// Manifest schema version.
    pub spin_manifest_version: ManifestVersion<1>,

    /// Global application configuration.
    pub application: ApplicationConfig,

    /// Custom configuration variable definition.
    #[serde(default)]
    pub variables: spin_config::Tree,

    /// Trigger configurations, by trigger type.
    #[serde(rename = "trigger", with = "maps_duplicate_key_is_error")]
    pub triggers: HashMap<TriggerType, Vec<TriggerConfig>>,

    /// Component configurations.
    #[serde(rename = "component")]
    pub components: Vec<ComponentManifest>,
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationConfig {
    /// Name of the application.
    pub name: String,

    /// Version of the application.
    pub version: String,

    /// Description of the application.
    pub description: Option<String>,

    /// List of the authors of the application.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authors: Vec<String>,

    /// Application-global trigger configuration, by trigger type.
    #[serde(rename = "trigger", default, skip_serializing_if = "HashMap::is_empty")]
    pub trigger_configs: HashMap<TriggerType, TriggerConfig>,
}

/// A valid trigger type.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct TriggerType(String);

impl TriggerType {
    pub fn new(type_: &'static str) -> Self {
        type_.to_string().try_into().expect("invalid TriggerType")
    }
}

impl TryFrom<String> for TriggerType {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err(Error::InvalidTriggerType("empty".to_string()));
        }
        // TODO(lann): more constraints?
        Ok(Self(value))
    }
}

/// Trigger represents configuration for a single trigger instance.
pub type TriggerConfig = HashMap<String, Value>;

#[skip_serializing_none]
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentManifest {
    /// Component ID.
    pub id: ComponentId,

    /// Description of the component.
    pub description: Option<String>,

    /// Source for the WebAssembly module for the component.
    pub source: ComponentSource,

    /// Environment variables for the component's WASI environment.
    #[serde(
        with = "maps_duplicate_key_is_error",
        default,
        skip_serializing_if = "HashMap::is_empty"
    )]
    pub environment: HashMap<String, String>,

    /// Files for the component's WASI environment.
    #[serde(default)]
    pub files: Vec<FileMapping>,

    /// Custom configuration values.
    #[serde(
        with = "maps_duplicate_key_is_error",
        default,
        skip_serializing_if = "HashMap::is_empty"
    )]
    pub config: HashMap<String, String>,

    /// Component build configuration.
    pub build: Option<ComponentBuildConfig>,
}

/// A valid component ID.
#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct ComponentId(String);

impl ComponentId {
    pub fn new(id: &'static str) -> Self {
        id.to_string().try_into().expect("invalid component ID")
    }
}

impl AsRef<str> for ComponentId {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl TryFrom<String> for ComponentId {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err(Error::InvalidComponentId("empty".to_string()));
        }
        // TODO(lann): more constraints?
        Ok(Self(value))
    }
}

/// The source for a Component.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, untagged)]
pub enum ComponentSource {
    /// A local path to the entrypoint Wasm module.
    Local(PathBuf),

    /// A reference to a Bindle Parcel.
    // REVIEW: changed `reference` -> `bindle` for forward-compat
    Bindle { bindle: String, parcel: String },
}

/// Configuration for mapping a single file or glob pattern into a component environment.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, untagged)]
pub enum FileMapping {
    /// A single path or glob pattern to use as both source and destination.
    Pattern(String),

    /// A source and destination path pair.
    Placement {
        source: PathBuf,
        destination: PathBuf,
    },
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentBuildConfig {
    /// Build command.
    pub command: String,

    /// Working directory in which the build command is executed,
    /// relative to the directory in which `spin.toml` is located.
    pub workdir: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal() -> Manifest {
        toml::toml! {
            spin_manifest_version = 1

            [application]
            name = "minimal"
            version = "0.0.0"

            [[trigger.http]]
            component = "root"

            [[component]]
            id = "root"
            source = "root.wasm"
        }
        .try_into()
        .unwrap()
    }

    fn kitchen_sink() -> Manifest {
        toml::toml! {
            spin_manifest_version = 1

            [application]
            name = "kitchen sink"
            version = "9999.9.9"
            description = "Test All Features"
            authors = ["Charles Xavier <prof.x@xmen.io>"]
            [application.trigger.http]
            base = "/kitchen"

            [variables]
            site_title = { default = "Kitchen Sink" }
            password = { required = true }

            [[trigger.http]]
            component = "main-page"

            [[trigger.http]]
            route = "/admin"
            component = "admin-page"
            [trigger.http.executor]
            type = "wagi"
            entrypoint = "start"
            argv = "serve"

            [[component]]
            id = "main-page"
            description = "Main Page"
            source = "main.wasm"
            files = [
                "static/*",
                { source = "src", destination = "dst" },
            ]
            [component.config]
            title = "{{site_title}}"

            [[component]]
            id = "admin-page"
            source = { bindle = "my-bindle", parcel = "abc123" }
            [component.environment]
            ADMIN_PASSWORD = "{{password}}"
            [component.build]
            command = "make admin"
            workdir = "admin"
        }
        .try_into()
        .unwrap()
    }

    #[test]
    fn minimal_smoke_check() {
        minimal();
    }

    #[test]
    fn kitchen_sink_parses_correctly() {
        let manifest = kitchen_sink();

        let app = &manifest.application;
        assert_eq!(app.name, "kitchen sink");
        assert_eq!(app.version, "9999.9.9");
        assert_eq!(app.description.as_deref(), Some("Test All Features"));
        assert_eq!(app.authors, ["Charles Xavier <prof.x@xmen.io>"]);

        let http = TriggerType::new("http");
        assert_eq!(
            app.trigger_configs[&http]["base"],
            Value::String("/kitchen".to_string())
        );

        let trigger = &manifest.triggers[&http][1];
        assert_eq!(trigger["route"], Value::String("/admin".to_string()));
        assert_eq!(
            trigger["component"],
            Value::String("admin-page".to_string())
        );
        match &trigger["executor"] {
            Value::Map(ref executor) => {
                for (k, v) in [("type", "wagi"), ("entrypoint", "start"), ("argv", "serve")] {
                    assert_eq!(
                        executor[&Value::String(k.to_string())],
                        Value::String(v.to_string())
                    );
                }
            }
            wrong => panic!("wrong type: {:?}", wrong),
        }

        let main = &manifest.components[0];
        assert_eq!(main.id, ComponentId::new("main-page"));
        assert_eq!(main.description.as_deref(), Some("Main Page"));
        assert!(matches!(main.source, ComponentSource::Local(ref path)
            if path.as_os_str() == "main.wasm"));
        assert!(matches!(main.files[0], FileMapping::Pattern(ref pat)
            if pat == "static/*"));
        assert!(
            matches!(main.files[1], FileMapping::Placement { ref source, ref destination }
                if source.as_os_str() == "src" && destination.as_os_str() == "dst")
        );
        assert_eq!(main.config["title"], "{{site_title}}");

        let admin = &manifest.components[1];
        assert!(
            matches!(admin.source, ComponentSource::Bindle { ref bindle, ref parcel }
                if bindle == "my-bindle" && parcel == "abc123")
        );
        assert_eq!(admin.environment["ADMIN_PASSWORD"], "{{password}}");
        let build = admin.build.as_ref().unwrap();
        assert_eq!(build.command, "make admin");
        assert_eq!(build.workdir.as_ref().unwrap().as_os_str(), "admin");
    }
}
