//! Dynamically-typed value helpers.

use serde::Serialize;
use serde_json::Value;

/// A String-keyed map with dynamically-typed values.
pub type ValuesMap = serde_json::Map<String, Value>;

/// ValuesMapBuilder assists in building a ValuesMap.
#[derive(Default)]
pub struct ValuesMapBuilder(ValuesMap);

impl ValuesMapBuilder {
    /// Returns a new empty ValuesMapBuilder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a string value into the map.
    pub fn string(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.entry(key, value.into())
    }

    /// Inserts a string value into the map only if the given Option is Some.
    pub fn string_option(
        &mut self,
        key: impl Into<String>,
        value: Option<impl Into<String>>,
    ) -> &mut Self {
        if let Some(value) = value {
            self.0.insert(key.into(), value.into().into());
        }
        self
    }

    /// Inserts a string array into the map.
    pub fn string_array<T: Into<String>>(
        &mut self,
        key: impl Into<String>,
        iter: impl IntoIterator<Item = T>,
    ) -> &mut Self {
        self.entry(key, iter.into_iter().map(|s| s.into()).collect::<Vec<_>>())
    }

    /// Inserts an entry into the map using the value's `impl Into<Value>`.
    pub fn entry(&mut self, key: impl Into<String>, value: impl Into<Value>) -> &mut Self {
        self.0.insert(key.into(), value.into());
        self
    }

    /// Inserts an entry into the map using the value's `impl Serialize`.
    pub fn serializable(
        &mut self,
        key: impl Into<String>,
        value: impl Serialize,
    ) -> serde_json::Result<&mut Self> {
        let value = serde_json::to_value(value)?;
        self.0.insert(key.into(), value);
        Ok(self)
    }

    /// Returns the built ValuesMap.
    pub fn build(self) -> ValuesMap {
        self.0
    }

    /// Returns the build ValuesMap and resets the builder to empty.
    pub fn take(&mut self) -> ValuesMap {
        std::mem::take(&mut self.0)
    }
}
