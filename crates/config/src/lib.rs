//! Configuration of an application for the Spin runtime.

#![deny(missing_docs)]

use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

/// Configuration for an application.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Config {
    /// Name of the application.
    pub name: String,
    /// Version of the application.
    pub version: String,
    /// Description of the application.
    pub description: Option<String>,
    /// Authors of the application.
    pub authors: Option<Vec<String>>,
    /// Components of the application.
    pub component: Vec<Component>,
}

/// An application component.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Component {
    /// Path (relative to the configuration file) of the main
    /// Wasm module for the current component.
    /// Only used when packaging the application.
    pub path: Option<PathBuf>,
    /// Reference to a component from the registry.
    pub reference: Option<String>,
    /// Name of the component. Used at runtime to select between
    /// multiple components of the same application.
    pub name: String,
    /// Optional route for HTTP applications.
    pub route: Option<String>,
    /// Trigger for the component.
    pub trigger: String,
    /// Files to be mapped inside the Wasm module at runtime.
    pub files: Option<Vec<String>>,
    /// Optional list of HTTP hosts the component is allowed to connect.
    pub allowed_http_hosts: Option<Vec<String>>,
    /// Optional list of dependencies to be resolved at runtime by the host.
    pub dependencies: Option<HashMap<String, Dependency>>,
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
    pub reference: Option<String>,
}

/// The dependency type.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub enum DependencyType {
    /// Host dependency.
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
    
    [[component]]
        path    = "path/to/wasm/file.wasm"
        name    = "four-lights"
        trigger = "fermyon/http/0.1.0"
        route   = "/lights"
        files   = []
    [component.dependencies]
        cache    = { type = "host" }
        markdown = { type = "component", reference = "github/octo-markdown/1.0.0" }
    "#;

    #[test]
    fn test_config() -> Result<()> {
        let cfg: Config = toml::from_str(CFG_TEST)?;

        assert_eq!(cfg.name, "chain-of-command");
        assert_eq!(cfg.version, "6.11.2");
        assert_eq!(
            cfg.description,
            Some("A simple application that returns the number of lights".to_string())
        );
        assert_eq!(cfg.authors.unwrap().len(), 3);

        assert_eq!(
            cfg.component[0].path.clone().unwrap().to_str().unwrap(),
            "path/to/wasm/file.wasm"
        );
        assert_eq!(cfg.component[0].name, "four-lights".to_string());
        assert_eq!(cfg.component[0].trigger, "fermyon/http/0.1.0".to_string());
        assert_eq!(cfg.component[0].route, Some("/lights".to_string()));

        assert_eq!(
            cfg.component[0]
                .dependencies
                .clone()
                .unwrap()
                .get("cache")
                .unwrap()
                .dependency_type,
            DependencyType::Host
        );

        assert_eq!(
            cfg.component[0]
                .dependencies
                .clone()
                .unwrap()
                .get("markdown")
                .unwrap()
                .reference,
            Some("github/octo-markdown/1.0.0".to_string())
        );

        Ok(())
    }
}
