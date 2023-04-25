//! Spin key-value persistent storage
//!
//! This module provides a generic interface for key-value storage, which may be implemented by the host various
//! ways (e.g. via an in-memory table, a local file, or a remote database). Details such as consistency model and
//! durability will depend on the implementation and may vary from one to store to the next.

use super::wit::key_value;

use key_value::Store as RawStore;

/// Errors which may be raised by the methods of `Store`
pub type Error = key_value::Error;

/// Represents a store in which key value tuples may be placed
#[derive(Debug)]
pub struct Store(RawStore);

impl Store {
    /// Open the specified store.
    ///
    /// If `name` is empty, open the default store.  Other stores may also be available depending on the app
    /// configuration.
    pub fn open(name: impl AsRef<str>) -> Result<Self, Error> {
        key_value::open(name.as_ref()).map(Self)
    }

    /// Open the default store.
    ///
    /// This is equivalent to `Store::open("default")`.
    pub fn open_default() -> Result<Self, Error> {
        Self::open("default")
    }

    /// Get the value, if any, associated with the specified key in this store.
    ///
    /// If no value is found, this will return `Err(Error::NoSuchKey)`.
    pub fn get(&self, key: impl AsRef<str>) -> Result<Vec<u8>, Error> {
        key_value::get(self.0, key.as_ref())
    }

    /// Set the value for the specified key.
    ///
    /// This will overwrite any previous value, if present.
    pub fn set(&self, key: impl AsRef<str>, value: impl AsRef<[u8]>) -> Result<(), Error> {
        key_value::set(self.0, key.as_ref(), value.as_ref())
    }

    /// Delete the tuple for the specified key, if any.
    ///
    /// This will have no effect and return `Ok(())` if the tuple was not present.
    pub fn delete(&self, key: impl AsRef<str>) -> Result<(), Error> {
        key_value::delete(self.0, key.as_ref())
    }

    /// Check whether a tuple exists for the specified key.
    pub fn exists(&self, key: impl AsRef<str>) -> Result<bool, Error> {
        key_value::exists(self.0, key.as_ref())
    }

    /// Get the set of keys in this store.
    pub fn get_keys(&self) -> Result<Vec<String>, Error> {
        key_value::get_keys(self.0)
    }
}

impl Drop for Store {
    fn drop(&mut self) {
        key_value::close(self.0)
    }
}
