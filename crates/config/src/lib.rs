//! Configuration of an application for the Spin runtime.

#![deny(missing_docs)]

use anyhow::{Context, Result};
use path_absolutize::Absolutize;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

mod bindle_utils;
mod parser;
mod schema;

pub use bindle_utils::BindleReader;

/// Reads the application configuration from the specified file.
pub fn read_from_file(app_file: impl AsRef<Path>) -> Result<Configuration<CoreComponent>> {
    let mut buf = vec![];
    let mut file = File::open(&app_file)
        .with_context(|| format!("Failed to open configuration file '{}'", app_file.as_ref().display()))?;
    file.read_to_end(&mut buf)?;

    let absolute_app_path = app_file.as_ref().absolutize()?.into_owned();

    let manifest: schema::file::AppManifest = toml::from_slice(&buf)?;
    Ok(parser::file::parse(manifest, &absolute_app_path))
}

/// Reads the application configuration from the specified bindle.
pub async fn read_from_bindle(
    id: &bindle::Id,
    server_url: &str,
) -> Result<Configuration<CoreComponent>> {
    // TODO: provide a way to specify auth
    let token_manager = bindle_utils::BindleTokenManager::NoToken(bindle::client::tokens::NoToken);
    let client = bindle::client::Client::new(server_url, token_manager)
        .with_context(|| format!("Invalid Bindle server URL '{}'", server_url))?;
    let reader = BindleReader::remote(&client, &id);

    let invoice = reader
        .get_invoice()
        .await
        .with_context(|| format!("Failed to load invoice '{}' from '{}'", id, server_url))?;

    let manifest_id = bindle_utils::find_application_manifest(&invoice)
        .with_context(|| format!("Failed to find application manifest in '{}'", id))?;
    let manifest_content = reader.get_parcel(&manifest_id).await
        .with_context(|| format!("Failed to fetch manifest from server '{}'", server_url))?;
    let manifest: schema::parcel::AppManifest = toml::from_slice(&manifest_content)
        .context("Failed to parse application manifest")?;

    Ok(parser::bindle::parse(manifest, &invoice, &reader, server_url))
}

/// Application configuration.
#[derive(Clone, Debug)]
pub struct Configuration<T> {
    /// General application information.
    pub info: ApplicationInformation,

    /// Configuration for the application components.
    pub components: Vec<T>,
}

/// A local component, as defined in `spin.toml`, potentially
/// mutable to be distributed.
/// TODO
///
/// Find a better name.
#[derive(Clone, Debug)]
pub struct LinkableComponent {
    /// Common component configuration.
    pub core: CoreComponent,
    // TODO
    //
    // There should be subsections for various environments
    // (i.e. dependencies.local, dependencies.prod)
    /// Optional list of dependencies to be resolved at runtime by the host.
    pub dependencies: Option<HashMap<String, Dependency>>,
    /// Optional build information or configuration that could be used
    /// by a plugin to build the Wasm module.
    pub build: Option<BuildConfig>,
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
    pub trigger: ApplicationTrigger,
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
    /// The application was loaded from the specified bindle.
    Bindle(bindle::Id, String),
}

/// The trigger type.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase", tag = "type")]
pub enum ApplicationTrigger {
    /// HTTP trigger type.
    Http(HttpTriggerConfiguration),
}

impl Default for ApplicationTrigger {
    fn default() -> Self {
        Self::Http(HttpTriggerConfiguration::default())
    }
}

/// HTTP trigger configuration.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
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
    /// Files to be mapped inside the Wasm module at runtime.
    pub files: ReferencedFiles,
    /// Optional list of HTTP hosts the component is allowed to connect.
    pub allowed_http_hosts: Vec<String>,
}

/// The set of assets referenced by a component.
#[derive(Clone)]
pub enum ReferencedFiles {
    /// There are no asset references.
    None,
    /// The asset references are file patterns relative to the application
    /// directory.
    FilePatterns(PathBuf, Vec<String>),
    /// The asset references resolved to parcels.
    BindleParcels(BindleReader, bindle::Id, Vec<bindle::Label>),
}

impl std::fmt::Debug for ReferencedFiles {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None =>
                f.debug_tuple("None")
                    .finish(),
            Self::FilePatterns(path, patterns) =>
                f.debug_tuple("FilePatterns")
                    .field(path)
                    .field(patterns)
                    .finish(),
            Self::BindleParcels(_, invoice_id, labels) =>
                f.debug_tuple("BindleParcels")
                    // We can't provide any debug info about the client or token manager it seems?
                    .field(invoice_id)
                    .field(labels)
                    .finish(),
        }
    }
}

/// Source for the module.
#[derive(Clone, Debug)]
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
#[derive(Clone, Debug)]
pub struct BindleComponentSource {
    /// Reader for the specified bindle
    pub reader: bindle_utils::BindleReader,
    /// Parcel to use from the bindle.
    pub parcel: String,
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
/// The dependency type.
#[derive(Clone, Debug)]
pub enum Dependency {
    /// A host dependency.
    Host,
    /// A component dependency.
    Component(BindleComponentSource),
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;
    use anyhow::Result;

    const CFG_TEST: &str = r#"
    name        = "chain-of-command"
    version     = "6.11.2"
    description = "A simple application that returns the number of lights"
    authors     = [ "Gul Madred", "Edward Jellico", "JL" ]
    trigger     = { type = "http", base   = "/" }

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

    fn read_from_temp_file(toml_text: &str) -> Result<Configuration<CoreComponent>> {
        let mut f = tempfile::NamedTempFile::new()?;
        f.write_all(toml_text.as_bytes())?;
        let config = read_from_file(&f)?;
        drop(f);
        Ok(config)
    }

    #[test]
    fn test_local_config() -> Result<()> {
        let cfg = read_from_temp_file(CFG_TEST)?;

        assert_eq!(cfg.info.name, "chain-of-command");
        assert_eq!(cfg.info.version, "6.11.2");
        assert_eq!(
            cfg.info.description,
            Some("A simple application that returns the number of lights".to_string())
        );

        let ApplicationTrigger::Http(http) = cfg.info.trigger;
        assert_eq!(http.base, "/".to_string());

        assert_eq!(cfg.info.authors.len(), 3);
        assert_eq!(cfg.components[0].id, "four-lights".to_string());

        let TriggerConfig::Http(http) = cfg.components[0].trigger.clone();
        assert_eq!(http.executor.unwrap(), HttpExecutor::Spin);
        assert_eq!(http.route, "/lights".to_string());

        let test_component = &cfg.components[0];
        // TODO: restore
        // let test_deps = test_component.dependencies.as_ref().unwrap();
        let test_env = &test_component.wasm.environment;
        let test_files = match &test_component.wasm.files {
            ReferencedFiles::FilePatterns(_, fps) => fps,
            _ => {
                assert!(false, "Expected file patterns but got {:?}", test_component.wasm.files);
                panic!("Expected file patterns but got {:?}", test_component.wasm.files)
            },
        };

        // TODO: restore
        // assert_eq!(test_deps.get("cache").unwrap(), &Dependency::Host);
        // assert_eq!(
        //     test_deps.get("markdown").unwrap(),
        //     &Dependency::Component(BindleComponentSource {
        //         reference: "github/octo-markdown/1.0.0".to_string(),
        //         parcel: "md.wasm".to_string()
        //     })
        // );

        let b = match cfg.components[1].source.clone() {
            ModuleSource::Bindle(b) => b,
            ModuleSource::FileReference(_) => panic!("expected bindle source"),
            ModuleSource::Linked(_) => panic!("expected bindle source"),
        };

        // assert_eq!(b.reference, "bindle reference".to_string());
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
