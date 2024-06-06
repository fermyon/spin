//! Various Spin conformant runtimes

pub mod in_process_spin;
pub mod spin_cli;
pub mod spin_containerd_shim;

/// The type of app Spin is running
pub enum SpinAppType {
    /// Expect an http listener to start
    Http,
    /// Expect a redis listener to start
    Redis,
    /// Don't expect Spin to start
    None,
}
