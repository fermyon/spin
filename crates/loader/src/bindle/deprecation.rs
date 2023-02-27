use std::sync::Once;

/// Prints a notice that Bindle support is deprecated.
pub fn print_bindle_deprecation() {
    static PRINT: Once = Once::new();

    PRINT.call_once(|| {
        eprintln!("WARNING: Bindle support is deprecated and will be removed in a future version.");
        eprintln!("For remote applications, please use registry commands and features instead.");
        eprintln!("See https://developer.fermyon.com/spin/spin-oci for more details");
        eprintln!();
    });
}
