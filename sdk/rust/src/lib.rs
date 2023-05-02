//! The Rust Spin SDK.

// #![deny(missing_docs)]

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

mod wit {
    #![allow(missing_docs)]
    wit_bindgen::generate!({
        world: "spin",
        path: "../../wit/ephemeral",
        macro_call_prefix: "::spin_sdk::",
        duplicate_if_necessary,
        macro_export
    });
}

/// Needed by the export macro
///
/// See [this commit](https://github.com/bytecodealliance/wit-bindgen/pull/394/commits/9d2ea88f986f4a883ba243449e3a070cac18958e) for more info.
#[cfg(target_arch = "wasm32")]
#[doc(hidden)]
pub use wit::__link_section;

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
    use super::wit::redis_types;
    use std::hash::{Hash, Hasher};

    // Exports the generated outbound Redis items.
    pub use super::wit::outbound_redis::{
        del, execute, get, incr, publish, sadd, set, smembers, srem,
    };
    pub use redis_types::*;

    impl PartialEq for redis_types::RedisResult {
        fn eq(&self, other: &Self) -> bool {
            use redis_types::RedisResult::*;
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
            use redis_types::RedisResult::*;

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
    pub use super::wit::spin_config::{get_config as get, Error};
}

pub mod inbound_http {
    use super::wit;
    pub use wit::inbound_http::*;

    impl TryFrom<wit::inbound_http::Request> for http_types::Request<Option<bytes::Bytes>> {
        type Error = anyhow::Error;

        fn try_from(spin_req: wit::inbound_http::Request) -> Result<Self, Self::Error> {
            let mut http_req = http_types::Request::builder()
                .method(spin_req.method)
                .uri(&spin_req.uri);

            append_request_headers(&mut http_req, &spin_req)?;

            let body = match spin_req.body {
                Some(b) => b.to_vec(),
                None => Vec::new(),
            };

            let body = Some(bytes::Bytes::from(body));

            Ok(http_req.body(body)?)
        }
    }

    impl From<wit::inbound_http::Method> for http_types::Method {
        fn from(spin_method: wit::inbound_http::Method) -> Self {
            match spin_method {
                wit::inbound_http::Method::Get => http_types::Method::GET,
                wit::inbound_http::Method::Post => http_types::Method::POST,
                wit::inbound_http::Method::Put => http_types::Method::PUT,
                wit::inbound_http::Method::Delete => http_types::Method::DELETE,
                wit::inbound_http::Method::Patch => http_types::Method::PATCH,
                wit::inbound_http::Method::Head => http_types::Method::HEAD,
                wit::inbound_http::Method::Options => http_types::Method::OPTIONS,
            }
        }
    }

    fn append_request_headers(
        http_req: &mut http_types::request::Builder,
        spin_req: &wit::inbound_http::Request,
    ) -> anyhow::Result<()> {
        let headers = http_req.headers_mut().unwrap();
        for (k, v) in &spin_req.headers {
            headers.append(
                <http_types::header::HeaderName as std::str::FromStr>::from_str(k)?,
                http_types::header::HeaderValue::from_str(v)?,
            );
        }

        Ok(())
    }

    impl TryFrom<wit::inbound_http::Response> for http_types::Response<Option<bytes::Bytes>> {
        type Error = anyhow::Error;

        fn try_from(spin_res: wit::inbound_http::Response) -> Result<Self, Self::Error> {
            let mut http_res = http_types::Response::builder().status(spin_res.status);
            append_response_headers(&mut http_res, spin_res.clone())?;

            let body = match spin_res.body {
                Some(b) => b.to_vec(),
                None => Vec::new(),
            };
            let body = Some(bytes::Bytes::from(body));

            Ok(http_res.body(body)?)
        }
    }

    fn append_response_headers(
        http_res: &mut http_types::response::Builder,
        spin_res: wit::inbound_http::Response,
    ) -> anyhow::Result<()> {
        let headers = http_res.headers_mut().unwrap();
        for (k, v) in spin_res.headers.unwrap() {
            headers.append(
                <http_types::header::HeaderName as ::std::str::FromStr>::from_str(&k)?,
                http_types::header::HeaderValue::from_str(&v)?,
            );
        }

        Ok(())
    }

    impl TryFrom<http_types::Response<Option<bytes::Bytes>>> for wit::inbound_http::Response {
        type Error = anyhow::Error;

        fn try_from(
            http_res: http_types::Response<Option<bytes::Bytes>>,
        ) -> Result<Self, Self::Error> {
            let status = http_res.status().as_u16();
            let headers = Some(outbound_headers(http_res.headers())?);
            let body = http_res.body().as_ref().map(|b| b.to_vec());

            Ok(wit::inbound_http::Response {
                status,
                headers,
                body,
            })
        }
    }

    fn outbound_headers(hm: &http_types::HeaderMap) -> anyhow::Result<Vec<(String, String)>> {
        let mut res = Vec::new();

        for (k, v) in hm {
            res.push((
                k.as_str().to_string(),
                std::str::from_utf8(v.as_bytes())?.to_string(),
            ));
        }

        Ok(res)
    }
}

pub mod inbound_redis {
    pub use super::wit::inbound_redis::*;
}
