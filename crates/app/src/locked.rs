//! Spin lock file (spin.lock) serialization models.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{metadata::MetadataExt, values::ValuesMap};

/// A String-keyed map with deterministic serialization order.
pub type LockedMap<T> = std::collections::BTreeMap<String, T>;

/// A LockedApp represents a "fully resolved" Spin application.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LockedApp {
    /// Locked schema version
    pub spin_lock_version: FixedVersion<0>,
    /// Application metadata
    #[serde(default, skip_serializing_if = "ValuesMap::is_empty")]
    pub metadata: ValuesMap,
    /// Custom config variables
    #[serde(default, skip_serializing_if = "LockedMap::is_empty")]
    pub variables: LockedMap<Variable>,
    /// Application triggers
    pub triggers: Vec<LockedTrigger>,
    /// Application components
    pub components: Vec<LockedComponent>,
}

impl LockedApp {
    /// Deserializes a [`LockedApp`] from the given JSON data.
    pub fn from_json(contents: &[u8]) -> serde_json::Result<Self> {
        serde_json::from_slice(contents)
    }

    /// Serializes the [`LockedApp`] into JSON data.
    pub fn to_json(&self) -> serde_json::Result<Vec<u8>> {
        serde_json::to_vec_pretty(&self)
    }

    /// Deserializes typed metadata for this app.
    ///
    /// Returns `Ok(None)` if there is no metadata for the given `key` and an
    /// `Err` only if there _is_ a value for the `key` but the typed
    /// deserialization failed.
    pub fn get_metadata<'this, T: Deserialize<'this>>(
        &'this self,
        key: crate::MetadataKey<T>,
    ) -> crate::Result<Option<T>> {
        self.metadata.get_typed(key)
    }

    /// Deserializes typed metadata for this app.
    ///
    /// Like [`LockedApp::get_metadata`], but returns an error if there is
    /// no metadata for the given `key`.
    pub fn require_metadata<'this, T: Deserialize<'this>>(
        &'this self,
        key: crate::MetadataKey<T>,
    ) -> crate::Result<T> {
        self.metadata.require_typed(key)
    }
}

/// A LockedComponent represents a "fully resolved" Spin component.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LockedComponent {
    /// Application-unique component identifier
    pub id: String,
    /// Component metadata
    #[serde(default, skip_serializing_if = "ValuesMap::is_empty")]
    pub metadata: ValuesMap,
    /// Wasm source
    pub source: LockedComponentSource,
    /// WASI environment variables
    #[serde(default, skip_serializing_if = "LockedMap::is_empty")]
    pub env: LockedMap<String>,
    /// WASI filesystem contents
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<ContentPath>,
    /// Custom config values
    #[serde(default, skip_serializing_if = "LockedMap::is_empty")]
    pub config: LockedMap<String>,
}

/// A LockedComponentSource specifies a Wasm source.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LockedComponentSource {
    /// Wasm source content type (e.g. "application/wasm")
    pub content_type: String,
    /// Wasm source content specification
    #[serde(flatten)]
    pub content: ContentRef,
}

/// A ContentPath specifies content mapped to a WASI path.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContentPath {
    /// Content specification
    #[serde(flatten)]
    pub content: ContentRef,
    /// WASI mount path
    pub path: PathBuf,
}

/// A ContentRef represents content used by an application.
///
/// At least one of `source` or `digest` must be specified. Implementations may
/// require one or the other (or both).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ContentRef {
    /// A URI where the content can be accessed. Implementations may support
    /// different URI schemes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// The content itself, base64-encoded.
    ///
    /// NOTE: This is both an optimization for small content and a workaround
    /// for certain OCI implementations that don't support 0 or 1 byte blobs.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "serde_base64"
    )]
    pub inline: Option<Vec<u8>>,
    /// If set, the content must have the given SHA-256 digest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<String>,
}

/// A LockedTrigger specifies configuration for an application trigger.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LockedTrigger {
    /// Application-unique trigger identifier
    pub id: String,
    /// Trigger type (e.g. "http")
    pub trigger_type: String,
    /// Trigger-type-specific configuration
    pub trigger_config: Value,
}

/// A Variable specifies a custom configuration variable.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Variable {
    /// The variable's default value. If unset, the variable is required.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    /// If set, the variable's value may be sensitive and e.g. shouldn't be logged.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub secret: bool,
}

/// FixedVersion represents a schema version field with a const value.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(into = "usize", try_from = "usize")]
pub struct FixedVersion<const V: usize>;

impl<const V: usize> From<FixedVersion<V>> for usize {
    fn from(_: FixedVersion<V>) -> usize {
        V
    }
}

impl<const V: usize> TryFrom<usize> for FixedVersion<V> {
    type Error = String;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        if value != V {
            return Err(format!("invalid version {} != {}", value, V));
        }
        Ok(Self)
    }
}

mod serde_base64 {
    use std::borrow::Cow;

    use base64::{engine::GeneralPurpose, prelude::BASE64_STANDARD_NO_PAD, Engine};
    use serde::{de, Deserialize, Deserializer, Serializer};

    const BASE64: GeneralPurpose = BASE64_STANDARD_NO_PAD;

    pub fn serialize<S>(bytes: &Option<Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match bytes {
            Some(bytes) => serializer.serialize_str(&BASE64.encode(bytes)),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Vec<u8>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        match Option::<Cow<str>>::deserialize(deserializer)? {
            Some(s) => Ok(Some(BASE64.decode(s.as_ref()).map_err(de::Error::custom)?)),
            None => Ok(None),
        }
    }
}
