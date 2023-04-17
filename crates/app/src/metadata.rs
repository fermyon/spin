use std::marker::PhantomData;

use serde::Deserialize;
use serde_json::Value;

use crate::{values::ValuesMap, Error, Result};

/// MetadataKey is a handle to a typed metadata value.
pub struct MetadataKey<T = String> {
    key: &'static str,
    _phantom: PhantomData<T>,
}

impl<T> MetadataKey<T> {
    /// Creates a new MetadataKey.
    pub const fn new(key: &'static str) -> Self {
        Self {
            key,
            _phantom: PhantomData,
        }
    }
}

impl<T> Clone for MetadataKey<T> {
    fn clone(&self) -> Self {
        Self {
            key: self.key,
            _phantom: PhantomData,
        }
    }
}

impl<T> Copy for MetadataKey<T> {}

impl<T> AsRef<str> for MetadataKey<T> {
    fn as_ref(&self) -> &str {
        self.key
    }
}

impl<T> From<MetadataKey<T>> for String {
    fn from(value: MetadataKey<T>) -> Self {
        value.key.to_string()
    }
}

impl<T> std::fmt::Debug for MetadataKey<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.key)
    }
}

pub trait MetadataExt {
    fn get_value(&self, key: &str) -> Option<&Value>;

    fn get_typed<'a, T: Deserialize<'a>>(&'a self, key: MetadataKey<T>) -> Result<Option<T>> {
        self.get_value(key.as_ref())
            .map(T::deserialize)
            .transpose()
            .map_err(|err| {
                Error::MetadataError(format!("invalid metadata value for {key:?}: {err:?}"))
            })
    }

    fn require_typed<'a, T: Deserialize<'a>>(&'a self, key: MetadataKey<T>) -> Result<T> {
        self.get_typed(key)?
            .ok_or_else(|| Error::MetadataError(format!("missing required metadata {key:?}")))
    }
}

impl MetadataExt for ValuesMap {
    fn get_value(&self, key: &str) -> Option<&Value> {
        self.get(key)
    }
}
