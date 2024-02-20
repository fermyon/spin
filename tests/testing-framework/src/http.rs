//! Utilities for tests that function over HTTP

pub use reqwest::Method;
use std::collections::HashMap;

/// A request to the Spin server
pub struct Request<'a, B> {
    pub method: Method,
    pub uri: &'a str,
    pub headers: &'a [(&'a str, &'a str)],
    pub body: Option<B>,
}

impl<'a, 'b> Request<'a, &'b [u8]> {
    /// Create a new request with no headers or body
    pub fn new(method: Method, uri: &'a str) -> Self {
        Self {
            method,
            uri,
            headers: &[],
            body: None,
        }
    }
}

impl<'a, B> Request<'a, B> {
    /// Create a new request with headers and a body
    pub fn full(
        method: Method,
        uri: &'a str,
        headers: &'a [(&'a str, &'a str)],
        body: Option<B>,
    ) -> Self {
        Self {
            method,
            uri,
            headers,
            body,
        }
    }
}

/// A response from a Spin server
pub struct Response {
    status: u16,
    headers: HashMap<String, String>,
    chunks: Vec<Vec<u8>>,
}

impl Response {
    /// A response with no headers or body
    pub fn new(status: u16) -> Self {
        Self {
            status,
            headers: Default::default(),
            chunks: Default::default(),
        }
    }

    /// A response with headers and a body
    pub fn new_with_body(status: u16, chunks: impl IntoChunks) -> Self {
        Self {
            status,
            headers: Default::default(),
            chunks: chunks.into_chunks(),
        }
    }

    /// A response with headers and a body
    pub fn full(status: u16, headers: HashMap<String, String>, chunks: impl IntoChunks) -> Self {
        Self {
            status,
            headers,
            chunks: chunks.into_chunks(),
        }
    }

    /// The status code of the response
    pub fn status(&self) -> u16 {
        self.status
    }

    /// The headers of the response
    pub fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }

    /// The body of the response
    pub fn body(&self) -> Vec<u8> {
        self.chunks.iter().flatten().copied().collect()
    }

    /// The body of the response as chunks of bytes
    ///
    /// If the response is not stream this will be a single chunk equal to the body
    pub fn chunks(&self) -> &[Vec<u8>] {
        &self.chunks
    }

    /// The body of the response as a string
    pub fn text(&self) -> Result<String, std::string::FromUtf8Error> {
        String::from_utf8(self.body())
    }
}

pub trait IntoChunks {
    fn into_chunks(self) -> Vec<Vec<u8>>;
}

impl IntoChunks for Vec<Vec<u8>> {
    fn into_chunks(self) -> Vec<Vec<u8>> {
        self
    }
}

impl IntoChunks for Vec<u8> {
    fn into_chunks(self) -> Vec<Vec<u8>> {
        vec![self]
    }
}

impl IntoChunks for String {
    fn into_chunks(self) -> Vec<Vec<u8>> {
        vec![self.into_bytes()]
    }
}

impl IntoChunks for &str {
    fn into_chunks(self) -> Vec<Vec<u8>> {
        vec![self.as_bytes().into()]
    }
}
