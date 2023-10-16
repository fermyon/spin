//! Serialization types for the Spin manifest file format (spin.toml).

use serde::Deserialize;

/// Serialization types for the Spin manifest V1.
pub mod v1;
/// Serialization types for the Spin manifest V2.
pub mod v2;

// Types common between manifest versions. Re-exported from versioned modules
// to make them easier to split if necessary.
pub(crate) mod common;

#[derive(Deserialize)]
pub(crate) struct VersionProbe {
    #[serde(alias = "spin_version")]
    pub spin_manifest_version: toml::Value,
}
