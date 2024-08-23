use crate::module_info::ModuleInfo;

pub const EARLIEST_PROBABLY_SAFE_CLANG_VERSION: &str = "15.0.7";

/// This error represents the likely presence of the allocation bug fixed in
/// https://github.com/WebAssembly/wasi-libc/pull/377 in a Wasm module.
#[derive(Debug, PartialEq)]
pub struct WasiLibc377Bug {
    clang_version: Option<String>,
}

impl WasiLibc377Bug {
    /// Detects the likely presence of this bug.
    pub fn check(module_info: &ModuleInfo) -> Result<(), Self> {
        if module_info.probably_uses_wit_bindgen() {
            // Modules built with wit-bindgen are probably safe.
            return Ok(());
        }
        if let Some(clang_version) = &module_info.clang_version {
            // Clang/LLVM version is a good proxy for wasi-sdk
            // version; the allocation bug was fixed in wasi-sdk-18
            // and LLVM was updated to 15.0.7 in wasi-sdk-19.
            if let Some((major, minor, patch)) = parse_clang_version(clang_version) {
                let earliest_safe =
                    parse_clang_version(EARLIEST_PROBABLY_SAFE_CLANG_VERSION).unwrap();
                if (major, minor, patch) >= earliest_safe {
                    return Ok(());
                } else {
                    return Err(Self {
                        clang_version: Some(clang_version.clone()),
                    });
                };
            } else {
                tracing::warn!(
                    clang_version,
                    "Unexpected producers.processed-by.clang version"
                );
            }
        }
        // If we can't assert that the module uses wit-bindgen OR was compiled
        // with a new-enough wasi-sdk, conservatively assume it may be buggy.
        Err(Self {
            clang_version: None,
        })
    }
}

impl std::fmt::Display for WasiLibc377Bug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "This Wasm module may have been compiled with wasi-sdk version <19 which \
            contains a critical memory safety bug. For more information, see: \
            https://github.com/fermyon/spin/issues/2552"
        )
    }
}

impl std::error::Error for WasiLibc377Bug {}

fn parse_clang_version(ver: &str) -> Option<(u16, u16, u16)> {
    // Strip optional trailing detail after space
    let ver = ver.split(' ').next().unwrap();
    let mut parts = ver.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    Some((major, minor, patch))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasi_libc_377_detect() {
        for (wasm, safe) in [
            (r#"(module)"#, false),
            (
                r#"(module (func (export "cabi_realloc") (unreachable)))"#,
                true,
            ),
            (
                r#"(module (func (export "some_other_function") (unreachable)))"#,
                false,
            ),
            (
                r#"(module (@producers (processed-by "clang" "16.0.0 extra-stuff")))"#,
                true,
            ),
            (
                r#"(module (@producers (processed-by "clang" "15.0.7")))"#,
                true,
            ),
            (
                r#"(module (@producers (processed-by "clang" "15.0.6")))"#,
                false,
            ),
            (
                r#"(module (@producers (processed-by "clang" "14.0.0 extra-stuff")))"#,
                false,
            ),
            (
                r#"(module (@producers (processed-by "clang" "a.b.c")))"#,
                false,
            ),
        ] {
            eprintln!("WAT: {wasm}");
            let module = wat::parse_str(wasm).unwrap();
            let module_info = ModuleInfo::from_module(&module).unwrap();
            let detected = WasiLibc377Bug::check(&module_info);
            assert!(detected.is_ok() == safe, "{wasm} -> {detected:?}");
        }
    }
}
