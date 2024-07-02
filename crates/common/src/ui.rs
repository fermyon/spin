//! Functions supporting common UI behaviour and standards

use std::path::Path;

/// Renders a Path with double quotes. This is the standard
/// for displaying paths in Spin. It is preferred to the Debug
/// format because the latter doubles up backlashes on Windows.
pub fn quoted_path(path: impl AsRef<Path>) -> impl std::fmt::Display {
    format!("\"{}\"", path.as_ref().display())
}

/// An operation that may be interrupted using Ctrl+C. This is usable only
/// when Ctrl+C is handled via the ctrlc crate - otherwise Ctrl+C terminates
/// the program. But in such situations, this trait helps you to convert
/// the interrupt into a 'cancel' value which you can use to gracefully
/// exit just as if the interrupt had been allowed to go through.
pub trait Interruptible {
    /// The result type that captures the cancellation.
    type Result;

    /// Converts an interrupt error to a value representing cancellation.
    fn cancel_on_interrupt(self) -> Self::Result;
}

impl<T> Interruptible for Result<Option<T>, std::io::Error> {
    type Result = Self;
    fn cancel_on_interrupt(self) -> Self::Result {
        match self {
            Ok(opt) => Ok(opt),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => Ok(None),
            Err(e) => Err(e),
        }
    }
}

impl Interruptible for Result<bool, std::io::Error> {
    type Result = Self;
    fn cancel_on_interrupt(self) -> Self::Result {
        match self {
            Ok(b) => Ok(b),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => Ok(false),
            Err(e) => Err(e),
        }
    }
}

impl Interruptible for Result<String, std::io::Error> {
    type Result = Result<Option<String>, std::io::Error>;
    fn cancel_on_interrupt(self) -> Self::Result {
        match self {
            Ok(s) => Ok(Some(s)),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => Ok(None),
            Err(e) => Err(e),
        }
    }
}
