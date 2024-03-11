//! Serde serialization helpers

#![deny(missing_docs)]

pub mod base64;
pub mod id;
mod version;

pub use version::{FixedStringVersion, FixedVersion, FixedVersionBackwardCompatible};

/// A "kebab-case" identifier.
pub type KebabId = id::Id<'-'>;

/// A "snake_case" identifier.
pub type SnakeId = id::Id<'_'>;
