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

/// Fixed schema version 1; (de)serializes as string "1".
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub struct FixedStringVersion<const V: usize>;

impl<const V: usize> From<FixedStringVersion<V>> for String {
    fn from(_: FixedStringVersion<V>) -> String {
        V.to_string()
    }
}

impl<const V: usize> TryFrom<String> for FixedStringVersion<V> {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.parse() != Ok(V) {
            return Err(format!("invalid version {value:?} != \"{V}\""));
        }
        Ok(Self)
    }
}
