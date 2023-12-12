//! Functions supporting common UI behaviour and standards

use std::path::Path;

/// Renders a Path with double quotes. This is the standard
/// for displaying paths in Spin. It is preferred to the Debug
/// format because the latter doubles up backlashes on Windows.
pub fn quoted_path(path: impl AsRef<Path>) -> impl std::fmt::Display {
    format!("\"{}\"", path.as_ref().display())
}
