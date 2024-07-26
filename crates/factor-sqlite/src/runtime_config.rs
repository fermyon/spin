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
