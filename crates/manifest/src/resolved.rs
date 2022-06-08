use std::{collections::BTreeMap, path::PathBuf};

use serde::{Deserialize, Serialize};
use serde_value::Value;
use serde_with::{hex::Hex, As};

use crate::ManifestVersion;

/// ResolvedManifest is a "resolved" application manifest, ready to be used by an executor.
//
// NOTE: The serialization of this data may be used for application de-duplication, so it
// should be kept reasonably stable, at least for a particular Spin release.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedManifest {
    /// Manifest schema version.
    pub spin_lock_version: ManifestVersion<0>,

    /// Application metadata, for logging, debugging, etc.
    pub metadata: ApplicationMetadata,

    /// Custom configuration variables.
    pub variables: BTreeMap<String, Variable>,

    /// Per-trigger-type configuration. Must have an entry for every trigger type used by the application.
    #[serde(default)]
    pub trigger_types: BTreeMap<String, BTreeMap<String, Value>>,

    /// Trigger definitions.
    pub triggers: Vec<Trigger>,

    /// Component definitions.
    pub components: Vec<Component>,
}

/// Application metadata; used for logging debugging, etc.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplicationMetadata {
    /// Application name.
    pub name: String,

    /// Application version.
    pub version: String,
}

/// Custom configuration variable definition.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Variable {
    /// True if variable is required.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub required: bool,

    /// Default value.
    pub default: Option<String>,
}

/// Trigger definition.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Trigger {
    /// Trigger type.
    #[serde(rename = "type")]
    type_: String,

    /// Trigger ID.
    id: Option<String>,

    /// Trigger-type-specific configuration.
    trigger_config: BTreeMap<String, Value>,
}

/// Component definition.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Component {
    /// Component ID.
    id: String,

    /// Component Wasm content reference.
    wasm: ContentRef,

    /// WASI configuration.
    #[serde(default)]
    wasi: WasiConfig,

    /// Custom configuration.
    config: BTreeMap<String, ConfigValue>,
}

/// A ContentRef is a content-addressed reference to data.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentRef {
    /// The content's digest.
    content_digest: ContentDigest,

    /// The content MIME type.
    content_type: Option<String>,
}

/// WASI configuration.
#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WasiConfig {
    /// File mounts.
    #[serde(default)]
    files: Vec<FileMount>,

    /// Environment variables.
    #[serde(default)]
    env: BTreeMap<String, String>,
}

/// A FileMount represents a file mounted into a WASI environment.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileMount {
    /// The file's content digest.
    content_digest: ContentDigest,

    /// The WASI environment path to mount
    path: PathBuf,
}

/// A FileMount represents a file mounted into a WASI environment.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContentDigest {
    Sha256(#[serde(with = "As::<Hex>")] Vec<u8>),
}

/// A ConfigValue represents a custom configuration value or template.
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConfigValue {
    Literal(String),
    Template(Vec<ConfigTemplatePart>),
}

/// A ConfigTemplatePart represents a literal or variable part of a config template.
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConfigTemplatePart {
    Literal(String),
    Var { var: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example() -> ResolvedManifest {
        serde_json::from_value(serde_json::json!({
            "spin_lock_version": 0,
            "metadata": {"name": "example-app", "version": "1.2.3"},
            "variables": {
                "req_var": { "required": true },
                "opt_var": { "default": "def" },
            },
            "trigger_types": {"http": {"base": "/base"}},
            "triggers": [
                {
                    "type": "http",
                    "id": "http-one",
                    "trigger_config": {
                        "component": "one",
                        "route": "/one",
                        "executor": { "type": "spin" }
                    }
                },
            ],
            "components": [
                {
                    "id": "one",
                    "wasm": {
                        "content_digest": {"sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"},
                        "content_type": "application/wasm",
                    },
                    "wasi": {
                        "files": [
                            {
                                "content_digest": {"sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"},
                                "path": "/data.json",
                            }
                        ],
                        "env": {
                            "VAR": "val"
                        }
                    },
                    "config": {
                        "simple": "value",
                        "complex": ["prefix-", {"var": "req_var"}, "-suffix"]
                    }
                },
            ],
        })).unwrap()
    }

    #[test]
    fn example_smoke_test() {
        example();
    }

    // NOTE: If you _intentionally_ changed the example serialization, replace
    // this with the new hash from the test failure output.
    const STABLE_SERIALIZATION_SHA256: &str =
        "470179e2350fdfe789546dcdf8a80800a9b74677cc02eb19181acd21ec5e16d4";

    #[test]
    fn stable_serialization() {
        let manifest = example();
        let mut hasher = <sha2::Sha256 as sha2::Digest>::new();
        serde_json::to_writer(&mut hasher, &manifest).unwrap();
        let hash = format!("{:x}", sha2::Digest::finalize(hasher));
        assert_eq!(hash, STABLE_SERIALIZATION_SHA256);
    }
}
