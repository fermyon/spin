//! Package for working with Wasm component templates.

#![deny(missing_docs)]

mod constraints;
mod directory;
mod environment;
mod filters;
mod interaction;
mod manager;
mod reader;
mod run;
mod source;
mod store;
mod template;

pub use manager::*;
pub use run::{Run, RunOptions, TemplatePreparationResult};
pub use source::TemplateSource;
pub use template::Template;
