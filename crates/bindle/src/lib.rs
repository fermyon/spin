//! Functions for publishing Spin applications to Bindle.
#![deny(missing_docs)]

mod error;
mod expander;
mod pusher;
mod writer;

pub use error::{PublishError, PublishResult};
pub use expander::expand_manifest;
pub use pusher::push_all;
pub use writer::{prepare_bindle, write};
