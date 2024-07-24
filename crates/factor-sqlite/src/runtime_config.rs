#[cfg(feature = "spin-cli")]
pub mod spin;

use std::{collections::HashMap, sync::Arc};

use crate::ConnectionPool;

/// A runtime configuration for SQLite databases.
///
/// Maps database labels to connection pools.
pub struct RuntimeConfig {
    pub pools: HashMap<String, Arc<dyn ConnectionPool>>,
}

/// Resolves a label to a default connection pool.
pub trait DefaultLabelResolver: Send + Sync {
    /// If there is no runtime configuration for a given database label, return a default connection pool.
    ///
    /// If `Option::None` is returned, the database is not allowed.
    fn default(&self, label: &str) -> Option<Arc<dyn ConnectionPool>>;
}
