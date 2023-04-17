//! The Rust Spin SDK.

#![deny(missing_docs)]

/// Outbound HTTP request functionality.
pub mod outbound_http;

/// Key/Value storage.
pub mod key_value;

/// Exports the procedural macros for writing handlers for Spin components.
pub use spin_macro::*;

#[export_name = concat!("spin-sdk-version-", env!("SDK_VERSION"))]
extern "C" fn __spin_sdk_version() {}

#[cfg(feature = "export-sdk-language")]
#[export_name = "spin-sdk-language-rust"]
extern "C" fn __spin_sdk_language() {}

#[export_name = concat!("spin-sdk-commit-", env!("SDK_COMMIT"))]
extern "C" fn __spin_sdk_hash() {}

/// Helpers for building Spin HTTP components.
/// These are convenience helpers, and the types in this module are
/// based on the [`http`](https://crates.io/crates) crate.
pub mod http {
    use anyhow::Result;

    /// The Spin HTTP request.
    pub type Request = http_types::Request<Option<bytes::Bytes>>;

    /// The Spin HTTP response.
    pub type Response = http_types::Response<Option<bytes::Bytes>>;

    pub use crate::outbound_http::send_request as send;

    /// Exports HTTP Router items.
    pub use router::*;
    mod router;

    /// Helper function to return a 404 Not Found response.
    pub fn not_found() -> Result<Response> {
        Ok(http_types::Response::builder()
            .status(404)
            .body(Some("Not Found".into()))?)
    }

    /// Helper function to return a 500 Internal Server Error response.
    pub fn internal_server_error() -> Result<Response> {
        Ok(http_types::Response::builder()
            .status(500)
            .body(Some("Internal Server Error".into()))?)
    }
}

/// Implementation of the spin redis interface.
#[allow(missing_docs)]
pub mod redis {
    use std::hash::{Hash, Hasher};

    wit_bindgen_rust::import!("../../wit/ephemeral/outbound-redis.wit");

    /// Exports the generated outbound Redis items.
    pub use outbound_redis::*;

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
    wit_bindgen_rust::import!("../../wit/ephemeral/spin-config.wit");

    /// Exports the generated Spin config items.
    pub use spin_config::{get_config as get, Error};

    impl ::std::fmt::Display for Error {
        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
            match self {
                Error::Provider(provider_err) => write!(f, "provider error: {}", provider_err),
                Error::InvalidKey(invalid_key) => write!(f, "invalid key: {}", invalid_key),
                Error::InvalidSchema(invalid_schema) => {
                    write!(f, "invalid schema: {}", invalid_schema)
                }
                Error::Other(other) => write!(f, "other: {}", other),
            }
        }
    }

    impl ::std::error::Error for Error {}
}
