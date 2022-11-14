#![deny(missing_docs)]

//! Functions for publishing Spin applications to Bindle.

mod bindle_pusher;
mod bindle_writer;
mod expander;

pub use bindle_pusher::push_all;
pub use bindle_writer::prepare_bindle;

use anyhow::Result;
use std::path::{Path, PathBuf};

pub(crate) fn app_dir(app_file: impl AsRef<Path>) -> Result<PathBuf> {
    let path_buf = app_file
        .as_ref()
        .parent()
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Failed to get containing directory for app file '{}'",
                app_file.as_ref().display()
            )
        })?
        .to_owned();
    Ok(path_buf)
}
