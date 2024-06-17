use anyhow::bail;
use wasm_metadata::Producers;
use wasmparser::{Encoding, ExternalKind, Parser, Payload};

/// Represents the detected likelihood of the allocation bug fixed in
/// https://github.com/WebAssembly/wasi-libc/pull/377 being present in a Wasm
/// module.
#[derive(Debug, PartialEq)]
pub enum WasiLibc377Bug {
    ProbablySafe,
    ProbablyUnsafe,
    Unknown,
}

impl WasiLibc377Bug {
    pub fn detect(module: &[u8]) -> anyhow::Result<Self> {
        for payload in Parser::new(0).parse_all(module) {
            match payload? {
                Payload::Version { encoding, .. } if encoding != Encoding::Module => {
                    bail!("detection only applicable to modules");
                }
                Payload::ExportSection(reader) => {
                    for export in reader {
                        let export = export?;
                        if export.kind == ExternalKind::Func && export.name == "cabi_realloc" {
                            // `cabi_realloc` is a good signal that this module
                            // uses wit-bindgen, making it probably-safe.
                            tracing::debug!("Found cabi_realloc export");
                            return Ok(Self::ProbablySafe);
                        }
                    }
                }
                Payload::CustomSection(c) if c.name() == "producers" => {
                    let producers = Producers::from_bytes(c.data(), c.data_offset())?;
                    if let Some(clang_version) =
                        producers.get("processed-by").and_then(|f| f.get("clang"))
                    {
                        tracing::debug!(clang_version, "Parsed producers.processed-by.clang");

                        // Clang/LLVM version is a good proxy for wasi-sdk
                        // version; the allocation bug was fixed in wasi-sdk-18
                        // and LLVM was updated to 15.0.7 in wasi-sdk-19.
                        if let Some((major, minor, patch)) = parse_clang_version(clang_version) {
                            return if (major, minor, patch) >= (15, 0, 7) {
                                Ok(Self::ProbablySafe)
                            } else {
                                Ok(Self::ProbablyUnsafe)
                            };
                        } else {
                            tracing::warn!(
                                clang_version,
                                "Unexpected producers.processed-by.clang version"
                            );
                        }
                    }
                }
                _ => (),
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
                ProbablyUnsafe,
            ),
            (
                r#"(module (@producers (processed-by "clang" "14.0.0")))"#,
                ProbablyUnsafe,
            ),
            (
                r#"(module (@producers (processed-by "clang" "a.b.c")))"#,
                Unknown,
            ),
        ] {
            eprintln!("WAT: {wasm}");
            let module = wat::parse_str(wasm).unwrap();
            let detected = WasiLibc377Bug::detect(&module).unwrap();
            assert_eq!(detected, expected);
        }
    }
}
