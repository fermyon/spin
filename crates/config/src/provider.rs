use std::fmt::Debug;

use crate::Key;

/// Environment variable based provider.
pub mod env;

/// A config provider.
pub trait Provider: Debug + Send + Sync {
    /// Returns the value at the given config path, if it exists.
    fn get(&self, key: &Key) -> anyhow::Result<Option<String>>;
}
