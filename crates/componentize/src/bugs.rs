use crate::module_info::ModuleInfo;

pub const EARLIEST_PROBABLY_SAFE_CLANG_VERSION: &str = "15.0.7";

/// Represents the detected likelihood of the allocation bug fixed in
/// https://github.com/WebAssembly/wasi-libc/pull/377 being present in a Wasm
/// module.
#[derive(Debug, PartialEq)]
pub enum WasiLibc377Bug {
    ProbablySafe,
    ProbablyUnsafe { clang_version: String },
    Unknown,
}

impl WasiLibc377Bug {
    pub fn detect(module_info: &ModuleInfo) -> anyhow::Result<Self> {
        if module_info.probably_uses_wit_bindgen() {
            // Modules built with wit-bindgen are probably safe.
            return Ok(Self::ProbablySafe);
        }
        if let Some(clang_version) = &module_info.clang_version {
            // Clang/LLVM version is a good proxy for wasi-sdk
            // version; the allocation bug was fixed in wasi-sdk-18
            // and LLVM was updated to 15.0.7 in wasi-sdk-19.
            if let Some((major, minor, patch)) = parse_clang_version(clang_version) {
                let earliest_safe =
                    parse_clang_version(EARLIEST_PROBABLY_SAFE_CLANG_VERSION).unwrap();
                return if (major, minor, patch) >= earliest_safe {
                    Ok(Self::ProbablySafe)
                } else {
                    Ok(Self::ProbablyUnsafe {
                        clang_version: clang_version.clone(),
                    })
                };
            } else {
                tracing::warn!(
                    clang_version,
                    "Unexpected producers.processed-by.clang version"
                );
            }
        }
        Ok(Self::Unknown)
    }
}

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
        use WasiLibc377Bug::*;
        for (wasm, expected) in [
            (r#"(module)"#, Unknown),
            (
                r#"(module (func (export "cabi_realloc") (unreachable)))"#,
                ProbablySafe,
            ),
            (
                r#"(module (func (export "some_other_function") (unreachable)))"#,
                Unknown,
            ),
            (
                r#"(module (@producers (processed-by "clang" "16.0.0 extra-stuff")))"#,
                ProbablySafe,
            ),
            (
                r#"(module (@producers (processed-by "clang" "15.0.7")))"#,
                ProbablySafe,
            ),
            (
                r#"(module (@producers (processed-by "clang" "15.0.6")))"#,
                ProbablyUnsafe {
                    clang_version: "15.0.6".into(),
                },
            ),
            (
                r#"(module (@producers (processed-by "clang" "14.0.0 extra-stuff")))"#,
                ProbablyUnsafe {
                    clang_version: "14.0.0 extra-stuff".into(),
                },
            ),
            (
                r#"(module (@producers (processed-by "clang" "a.b.c")))"#,
                Unknown,
            ),
        ] {
            eprintln!("WAT: {wasm}");
            let module = wat::parse_str(wasm).unwrap();
            let module_info = ModuleInfo::from_module(&module).unwrap();
            let detected = WasiLibc377Bug::detect(&module_info).unwrap();
            assert_eq!(detected, expected);
        }
    }
}
