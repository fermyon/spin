//! The Rust Spin SDK.

#![deny(missing_docs)]

/// Key/Value storage.
pub mod key_value;

/// SQLite storage.
pub mod sqlite;

/// Large Language Model APIs
pub mod llm;

/// Exports the procedural macros for writing handlers for Spin components.
pub use spin_macro::*;

#[doc(hidden)]
/// Module containing wit bindgen generated code.
///
/// This is only meant for internal consumption.
pub mod wit {
    #![allow(missing_docs)]

    wit_bindgen::generate!({
        world: "platform",
        path: "../../wit/preview2",
    });
    pub use fermyon::spin as v1;
    pub use fermyon::spin2_0_0 as v2;
}

/// Needed by the export macro
///
/// See [this commit](https://github.com/bytecodealliance/wit-bindgen/pull/394/commits/9d2ea88f986f4a883ba243449e3a070cac18958e) for more info.
#[cfg(target_arch = "wasm32")]
#[doc(hidden)]
pub use wit::__link_section;

#[export_name = concat!("spin-sdk-version-", env!("SDK_VERSION"))]
extern "C" fn __spin_sdk_version() {}

#[cfg(feature = "export-sdk-language")]
#[export_name = "spin-sdk-language-rust"]
extern "C" fn __spin_sdk_language() {}

#[export_name = concat!("spin-sdk-commit-", env!("SDK_COMMIT"))]
extern "C" fn __spin_sdk_hash() {}

/// Helpers for building Spin HTTP components.
pub mod http;

/// Implementation of the spin redis interface.
#[allow(missing_docs)]
pub mod redis {
    use std::hash::{Hash, Hasher};

    pub use super::wit::v1::redis::{del, execute, get, incr, publish, sadd, set, smembers, srem};
    pub use super::wit::v1::redis_types::*;

    impl PartialEq for RedisResult {
        fn eq(&self, other: &Self) -> bool {
            use RedisResult::*;
            match (self, other) {
                (Nil, Nil) => true,
                (Status(a), Status(b)) => a == b,
                (Int64(a), Int64(b)) => a == b,
                (Binary(a), Binary(b)) => a == b,
                _ => false,
            }
        }
    }

    impl Eq for RedisResult {}

    impl Hash for RedisResult {
        fn hash<H: Hasher>(&self, state: &mut H) {
            use RedisResult::*;

            match self {
                Nil => (),
                Status(s) => s.hash(state),
                Int64(v) => v.hash(state),
                Binary(v) => v.hash(state),
            }
        }
    }
}

/// Implementation of the spin postgres db interface.
pub mod pg;

/// Implementation of the Spin MySQL database interface.
pub mod mysql;

/// Implementation of the spin config interface.
#[allow(missing_docs)]
pub mod config {
    /// Exports the generated Spin config items.
    pub use super::wit::v1::config::{get_config as get, Error};
}

#[doc(hidden)]
pub use wit_bindgen;
