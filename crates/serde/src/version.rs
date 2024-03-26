use serde::{Deserialize, Serialize};

/// FixedVersion represents a version integer field with a const value.
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

/// FixedVersion represents a version integer field with a const value,
/// but accepts lower versions during deserialisation.
#[derive(Clone, Debug, Deserialize)]
#[serde(into = "usize", try_from = "usize")]
pub struct FixedVersionBackwardCompatible<const V: usize> {
    actual: usize,
}

impl<const V: usize> Default for FixedVersionBackwardCompatible<V> {
    fn default() -> Self {
        Self { actual: V }
    }
}

impl<const V: usize> From<FixedVersionBackwardCompatible<V>> for usize {
    fn from(_: FixedVersionBackwardCompatible<V>) -> usize {
        V
    }
}

impl<const V: usize> TryFrom<usize> for FixedVersionBackwardCompatible<V> {
    type Error = String;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        if value > V {
            return Err(format!("invalid version {} > {}", value, V));
        }
        Ok(Self { actual: value })
    }
}

impl<const V: usize> FixedVersionBackwardCompatible<V> {
    /// The underlying (downlevel, deserialised) version.
    pub fn actual(&self) -> usize {
        self.actual
    }
}

/// FixedStringVersion represents a version string field with a const value.
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
