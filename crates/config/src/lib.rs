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

impl<T> Configuration<T> {
    /// Derives a Configuration from the serialisation format.
    pub fn from_raw(raw: RawConfiguration<T>, origin: ApplicationOrigin) -> Self {
        Self {
            info: ApplicationInformation::from_raw(raw.info, origin),
            components: raw.components,
        }
    }
}

/// Application configuration file format.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RawConfiguration<T> {
    /// General application information.
    #[serde(flatten)]
    pub info: RawApplicationInformation,

    /// Configuration for the application components.
    #[serde(rename = "component")]
    pub components: Vec<T>,
}

/// A local component, as defined in `spin.toml`, potentially
/// mutable to be distributed.
/// TODO
///
/// Find a better name.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkableComponent {
    /// Common component configuration.
    #[serde(flatten)]
    pub core: CoreComponent,
    // TODO
    //
    // There should be subsections for various environments
    // (i.e. dependencies.local, dependencies.prod)
    /// Optional list of dependencies to be resolved at runtime by the host.
    pub dependencies: Option<HashMap<String, Dependency>>,
    /// Optional build information or configuration that could be used
    /// by a plugin to build the Wasm module.
    #[serde(rename = "build")]
    pub build: Option<BuildConfig>,
}

/// Core component configuration.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CoreComponent {
    /// The module source.
    pub source: ModuleSource,
    /// ID of the component. Used at runtime to select between
    /// multiple components of the same application.
    pub id: String,
    /// Per-component WebAssembly configuration.
    /// This takes precedence over the application-level
    /// WebAssembly configuration.
    #[serde(flatten)]
    pub wasm: WasmConfig,
    /// Trigger configuration.
    pub trigger: TriggerConfig,
}

/// General application information.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RawApplicationInformation {
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
    pub trigger: TriggerType,
    /// TODO
    pub namespace: Option<String>,
}

/// General application information.
#[derive(Clone, Debug)]
pub struct ApplicationInformation {
    /// Name of the application.
    pub name: String,
    /// Version of the application.
    pub version: String,
    /// Description of the application.
    pub description: Option<String>,
    /// Authors of the application.
    pub authors: Vec<String>,
    /// Trigger for the application.
    ///
    /// Currently, all components of a given application must be
    /// invoked as a result of the same trigger "type".
    /// In the future, applications with mixed triggers might be allowed,
    /// but for now, a component with a different trigger must be part of
    /// a separate application.
    pub trigger: TriggerType,
    /// TODO
    pub namespace: Option<String>,
    /// The location from which the application is loaded.
    pub origin: ApplicationOrigin,
}

/// The location from which an application was loaded.
#[derive(Clone, Debug)]
pub enum ApplicationOrigin {
    /// The application was loaded from the specified file.
    File(PathBuf),
}

impl ApplicationInformation {
    /// Derives an ApplicationInformation from the serialisation format.
    pub fn from_raw(raw: RawApplicationInformation, origin: ApplicationOrigin) -> Self {
        Self {
            name: raw.name,
            version: raw.version,
            description: raw.description,
            authors: raw.authors.unwrap_or_default(),
            trigger: raw.trigger,
            namespace: raw.namespace,
            origin,
        }
    }
}

/// The trigger type.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub enum TriggerType {
    /// HTTP trigger type.
    Http,
}

impl Default for TriggerType {
    fn default() -> Self {
        Self::Http
    }
}

/// WebAssembly configuration.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct WasmConfig {
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
pub enum ModuleSource {
    /// Local path or parcel reference to a module that needs to be linked.
    FileReference(PathBuf),
    /// Reference to a remote bindle
    Bindle(BindleComponentSource),
    /// Local path to a linked module.
    /// This variant is manually created by the linker component,
    /// and cannot be directly used in configuration outside the linker.
    Linked(PathBuf),
}

/// A component source from Bindle.
/// This assumes access to the Bindle server.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BindleComponentSource {
    /// Reference to the bindle (name/version)
    pub reference: String,
    /// Parcel to use from the bindle.
    pub parcel: String,
}

impl Default for ModuleSource {
    fn default() -> Self {
        // TODO
        //
        // What does Default mean for a module source?
        Self::FileReference(PathBuf::new())
    }
}

/// Configuration for the HTTP trigger.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
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
/// TODO
///
/// These should be versioned.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
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

/// Information about building the component.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BuildConfig {}

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

/// Dependency for a component.
/// Each entry this map should correspond to exactly one
/// import module from the Wasm module.
///
/// Currently, this map should either contain an interface that
/// should be satisfied by the host (through a host implementation),
/// or an exact reference (*not* a version range) to a component from the registry.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Dependency {
    /// The dependency type.
    #[serde(rename = "type")]
    pub dependency_type: DependencyType,

    /// Reference to a component from the registry.
    #[serde(flatten)]
    pub reference: Option<BindleComponentSource>,
}

/// The dependency type.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub enum DependencyType {
    /// A host dependency.
    Host,
    /// A component dependency.
    Component,
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    const CFG_TEST: &str = r#"
    name        = "chain-of-command"
    version     = "6.11.2"
    description = "A simple application that returns the number of lights"
    authors     = [ "Gul Madred", "Edward Jellico", "JL" ]
    trigger     = "http"
    
    [[component]]
        source = "path/to/wasm/file.wasm"
        id     = "four-lights"
        files  = ["file.txt", "subdir/another.txt"]
    [component.trigger]
        route          = "/lights"
        executor       = "spin"
    [component.dependencies]
        cache    = { type = "host" }
        markdown = { type = "component", reference = "github/octo-markdown/1.0.0", parcel = "md.wasm" }
    [component.environment]
        env1 = "first"
        env2 = "second"
    
    [[component]]
        id = "abc"
    [component.source]
        reference = "bindle reference"
        parcel    = "parcel"
    [component.trigger]
        route = "/test"
    "#;

    #[test]
    fn test_local_config() -> Result<()> {
        let cfg: RawConfiguration<LinkableComponent> = toml::from_str(CFG_TEST)?;

        assert_eq!(cfg.info.name, "chain-of-command");
        assert_eq!(cfg.info.version, "6.11.2");
        assert_eq!(
            cfg.info.description,
            Some("A simple application that returns the number of lights".to_string())
        );
        assert_eq!(cfg.info.authors.unwrap().len(), 3);
        assert_eq!(cfg.components[0].core.id, "four-lights".to_string());

        let TriggerConfig::Http(http) = cfg.components[0].core.trigger.clone();
        assert_eq!(http.executor.unwrap(), HttpExecutor::Spin);
        assert_eq!(http.route, "/lights".to_string());

        let test_component = &cfg.components[0];
        let test_deps = test_component.dependencies.as_ref().unwrap();
        let test_env = test_component.core.wasm.environment.as_ref().unwrap();
        let test_files = test_component.core.wasm.files.as_ref().unwrap();

        assert_eq!(
            test_deps.get("cache").unwrap().dependency_type,
            DependencyType::Host
        );

        assert_eq!(
            test_deps.get("markdown").unwrap().reference,
            Some(BindleComponentSource {
                reference: "github/octo-markdown/1.0.0".to_string(),
                parcel: "md.wasm".to_string()
            })
        );

        let b = match cfg.components[1].core.source.clone() {
            ModuleSource::Bindle(b) => b,
            ModuleSource::FileReference(_) => panic!("expected bindle source"),
            ModuleSource::Linked(_) => panic!("expected bindle source"),
        };

        assert_eq!(b.reference, "bindle reference".to_string());
        assert_eq!(b.parcel, "parcel".to_string());

        assert_eq!(2, test_env.len());
        assert_eq!("first", test_env.get("env1").unwrap());
        assert_eq!("second", test_env.get("env2").unwrap());

        assert_eq!(2, test_files.len());
        assert_eq!("file.txt", test_files[0]);
        assert_eq!("subdir/another.txt", test_files[1]);

        Ok(())
    }
}
