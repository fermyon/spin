//! Build information for the Spin CLI.

/// The version of the Spin CLI.
pub const SPIN_VERSION: &str = env!("CARGO_PKG_VERSION");
/// The major version of the Spin CLI.
pub const SPIN_VERSION_MAJOR: &str = env!("CARGO_PKG_VERSION_MAJOR");
/// The minor version of the Spin CLI.
pub const SPIN_VERSION_MINOR: &str = env!("CARGO_PKG_VERSION_MINOR");
/// The patch version of the Spin CLI.
pub const SPIN_VERSION_PATCH: &str = env!("CARGO_PKG_VERSION_PATCH");
/// The pre-release version of the Spin CLI.
pub const SPIN_VERSION_PRE: &str = env!("CARGO_PKG_VERSION_PRE");
/// The build date of the Spin CLI.
pub const SPIN_BUILD_DATE: &str = env!("VERGEN_BUILD_DATE");
/// The commit hash of the Spin CLI.
pub const SPIN_COMMIT_SHA: &str = env!("VERGEN_GIT_SHA");
/// The commit date of the Spin CLI.
pub const SPIN_COMMIT_DATE: &str = env!("VERGEN_GIT_COMMIT_DATE");
/// The branch of the Spin CLI.
pub const SPIN_BRANCH: &str = env!("VERGEN_GIT_BRANCH");
/// The target triple of the Spin CLI.
pub const SPIN_TARGET_TRIPLE: &str = env!("VERGEN_CARGO_TARGET_TRIPLE");
/// The profile of the Spin CLI.
pub const SPIN_DEBUG: &str = env!("VERGEN_CARGO_DEBUG");
