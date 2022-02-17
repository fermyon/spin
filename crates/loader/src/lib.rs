//! Loaders for Spin applications.
//! This crate implements the possible application sources for Spin applications,
//! and includes functionality to convert the specific configuration (for example
//! local configuration files, or pulled from a Bindle) into Spin configuration that
//! can be consumed by the Spin execution context.
//!
//! This crate can be extended (or replaced entirely) to support additional loaders,
//! and any implementation that produces a `Configuration<CoreComponent>` is compatible
//! with the Spin execution context.

#![deny(missing_docs)]

mod assets;
mod local;

/// Load a Spin application configuration from a spin.toml manifest file.
pub use local::from_file;
