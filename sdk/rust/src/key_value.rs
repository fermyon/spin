//! Spin key-value persistent storage
//!
//! This module provides a generic interface for key-value storage, which may be implemented by the host various
//! ways (e.g. via an in-memory table, a local file, or a remote database). Details such as consistency model and
//! durability will depend on the implementation and may vary from one to store to the next.

use super::wit::fermyon::spin::key_value;

#[cfg(feature = "json")]
use serde::{de::DeserializeOwned, Serialize};

/// Errors which may be raised by the methods of `Store`
pub use key_value::Error;

/// A store
pub use key_value::Store;

impl Store {
    #[cfg(feature = "json")]
    /// Serialize the given data to JSON, then set it as the value for the specified `key`.
    pub fn set_json<T: Serialize>(
        &self,
        key: impl AsRef<str>,
        value: &T,
    ) -> Result<(), anyhow::Error> {
        Ok(self.set(key.as_ref(), &serde_json::to_vec(value)?)?)
    }

    #[cfg(feature = "json")]
    /// Deserialize an instance of type `T` from the value of `key`.
    pub fn get_json<T: DeserializeOwned>(&self, key: impl AsRef<str>) -> Result<T, anyhow::Error> {
        Ok(serde_json::from_slice(&self.get(key.as_ref())?)?)
    }
}
