//! Loaders for Spin applications.
//! This crate implements the possible application sources for Spin applications,
//! and includes functionality to convert the specific configuration (for example
//! local configuration files or from OCI) into Spin configuration that
//! can be consumed by the Spin execution context.
//!
//! This crate can be extended (or replaced entirely) to support additional loaders,
//! and any implementation that produces a `Application` is compatible
//! with the Spin execution context.

#![deny(missing_docs)]

mod assets;
pub mod cache;
mod common;
#[cfg(feature = "local")]
pub mod local;
mod validation;

/// Load a Spin application configuration from a spin.toml manifest file.
#[cfg(feature = "local")]
pub use local::from_file;

/// Maximum number of assets to process in parallel
pub(crate) const MAX_PARALLEL_ASSET_PROCESSING: usize = 16;

pub use assets::to_relative;
