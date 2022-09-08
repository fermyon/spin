//! Loaders for Spin applications.
//! This crate implements the possible application sources for Spin applications,
//! and includes functionality to convert the specific configuration (for example
//! local configuration files, or pulled from a Bindle) into Spin configuration that
//! can be consumed by the Spin execution context.
//!
//! This crate can be extended (or replaced entirely) to support additional loaders,
//! and any implementation that produces a `Application` is compatible
//! with the Spin execution context.

#![deny(missing_docs)]

mod assets;
pub mod bindle;
pub mod local;

/// Load a Spin application configuration from a spin.toml manifest file.
pub use local::from_file;

/// Load a Spin application configuration from Bindle.
pub use crate::bindle::from_bindle;

pub use crate::assets::file_sha256_digest_string;

/// Maximum number of assets to process in parallel
pub(crate) const MAX_PARALLEL_ASSET_PROCESSING: usize = 16;
