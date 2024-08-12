#[cfg(feature = "spin-cli")]
pub mod spin;

use std::{collections::HashMap, sync::Arc};

use crate::ConnectionCreator;

/// A runtime configuration for SQLite databases.
///
/// Maps database labels to connection creators.
pub struct RuntimeConfig {
    pub connection_creators: HashMap<String, Arc<dyn ConnectionCreator>>,
}
