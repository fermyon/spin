/// Traits for converting between the various types
pub mod conversions;

use std::collections::HashMap;

#[doc(inline)]
pub use conversions::IntoResponse;

use self::conversions::TryFromIncomingResponse;

#[doc(inline)]
pub use super::wit::wasi::http::types::*;

/// A unified request object that can represent both incoming and outgoing requests.
///
/// This should be used in favor of `IncomingRequest` and `OutgoingRequest` when there
/// is no need for streaming bodies.
pub struct Request {
    /// The method of the request
    method: Method,
    /// The uri for the request
    ///
    /// The first item is set to `None` if the supplied uri is malformed
    uri: (Option<hyperium::Uri>, String),
    /// The request headers
    headers: HashMap<String, HeaderValue>,
    /// The request body as bytes
    body: Vec<u8>,
}

enum HeaderValue {
    String(String),
    Bytes(Vec<u8>),
}

impl HeaderValue {
    /// Turn the `HeaderValue` into bytes
    fn into_bytes(self) -> Vec<u8> {
        match self {
            HeaderValue::String(s) => s.into_bytes(),
            HeaderValue::Bytes(b) => b,
        }
    }
}

impl Request {
    fn new(method: Method, uri: impl Into<String>) -> Self {
        Self {
            method,
            uri: Self::parse_uri(uri.into()),
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    /// The request method
    pub fn method(&self) -> &Method {
        &self.method
    }

    /// The request uri
    pub fn uri(&self) -> &str {
        &self.uri.1
    }

    /// The request uri path
    pub fn path(&self) -> &str {
        self.uri.0.as_ref().map(|u| u.path()).unwrap_or_default()
    }

    /// The request uri query
    pub fn query(&self) -> &str {
        self.uri
            .0
            .as_ref()
            .and_then(|u| u.query())
            .unwrap_or_default()
    }

    /// The request headers
    ///
    /// This only returns headers that are utf8 encoded
    pub fn headers(&self) -> impl Iterator<Item = (&str, &str)> {
        self.headers.iter().filter_map(|(k, v)| match v {
            HeaderValue::String(v) => Some((k.as_str(), v.as_str())),
            HeaderValue::Bytes(_) => None,
        })
    }

    /// Return a header value
    ///
    /// Will return `None` if the header does not exist or if it is not utf8
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .get(&name.to_lowercase())
            .and_then(|v| match v {
                HeaderValue::String(s) => Some(s.as_str()),
                HeaderValue::Bytes(_) => None,
            })
    }

    /// The request headers as bytes
    pub fn headers_raw(&self) -> impl Iterator<Item = (&str, &[u8])> {
        self.headers.iter().map(|(k, v)| match v {
            HeaderValue::String(v) => (k.as_str(), v.as_bytes()),
            HeaderValue::Bytes(v) => (k.as_str(), v.as_slice()),
        })
    }

    /// The request body
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// The request body
    pub fn body_mut(&mut self) -> &mut Vec<u8> {
        &mut self.body
    }

    /// Consume this type and return its body
    pub fn into_body(self) -> Vec<u8> {
        self.body
    }

    /// Create a request builder
    pub fn builder() -> RequestBuilder {
        RequestBuilder::new(Method::Get, "/")
    }

    fn parse_uri(uri: String) -> (Option<hyperium::Uri>, String) {
        (
            hyperium::Uri::try_from(&uri)
                .or_else(|_| hyperium::Uri::try_from(&format!("http://{uri}")))
                .ok(),
            uri,
        )
    }
}

/// A request builder
pub struct RequestBuilder {
    request: Request,
}

impl RequestBuilder {
    /// Create a new `RequestBuilder`
    pub fn new(method: Method, uri: impl Into<String>) -> Self {
        Self {
            request: Request::new(method, uri.into()),
        }
    }

    /// Set the method
    pub fn method(&mut self, method: Method) -> &mut Self {
        self.request.method = method;
        self
    }

    /// Set the uri
    pub fn uri(&mut self, uri: impl Into<String>) -> &mut Self {
        self.request.uri = Request::parse_uri(uri.into());
        self
    }

    /// Set the headers
    pub fn headers(&mut self, headers: impl conversions::IntoHeaders) -> &mut Self {
        self.request.headers = into_header_rep(headers);
        self
    }

    /// Set a header
    pub fn header(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.request
            .headers
            .insert(key.into().to_lowercase(), HeaderValue::String(value.into()));
        self
    }

    /// Set the body
    pub fn body(&mut self, body: impl conversions::IntoBody) -> &mut Self {
        self.request.body = body.into_body();
        self
    }

    /// Build the `Request`
    pub fn build(&mut self) -> Request {
        std::mem::replace(&mut self.request, Request::new(Method::Get, "/"))
    }
}

/// A unified response object that can represent both outgoing and incoming responses.
///
/// This should be used in favor of `OutgoingResponse` and `IncomingResponse` when there
/// is no need for streaming bodies.
pub struct Response {
    /// The status of the response
    status: StatusCode,
    /// The response headers
    headers: HashMap<String, HeaderValue>,
    /// The body of the response as bytes
    body: Vec<u8>,
}

impl Response {
    /// Create a new response from a status and optional headers and body
    pub fn new(status: impl conversions::IntoStatusCode, body: impl conversions::IntoBody) -> Self {
        Self {
            status: status.into_status_code(),
            headers: HashMap::new(),
            body: body.into_body(),
        }
    }

    /// The response status
    pub fn status(&self) -> &StatusCode {
        &self.status
    }

    /// The response headers
    ///
    /// This only returns headers that are utf8 encoded
    pub fn headers(&self) -> impl Iterator<Item = (&str, &str)> {
        self.headers.iter().filter_map(|(k, v)| match v {
            HeaderValue::String(v) => Some((k.as_str(), v.as_str())),
            HeaderValue::Bytes(_) => None,
        })
    }

    /// Return a header value
    ///
    /// Will return `None` if the header does not exist or if it is not utf8
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| match v {
            HeaderValue::String(s) => Some(s.as_str()),
            HeaderValue::Bytes(_) => None,
        })
    }

    /// The request headers as bytes
    pub fn headers_raw(&self) -> impl Iterator<Item = (&str, &[u8])> {
        self.headers.iter().map(|(k, v)| match v {
            HeaderValue::String(v) => (k.as_str(), v.as_bytes()),
            HeaderValue::Bytes(v) => (k.as_str(), v.as_slice()),
        })
    }

    /// The response body
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// The response body
    pub fn body_mut(&mut self) -> &mut Vec<u8> {
        &mut self.body
    }

    /// Consume this type and return its body
    pub fn into_body(self) -> Vec<u8> {
        self.body
    }

    fn builder() -> ResponseBuilder {
        ResponseBuilder::new(200)
    }
}

/// A builder for `Response``
pub struct ResponseBuilder {
    response: Response,
}

impl ResponseBuilder {
    /// Create a new `ResponseBuilder`
    pub fn new(status: impl conversions::IntoStatusCode) -> Self {
        ResponseBuilder {
            response: Response::new(status, Vec::new()),
        }
    }

    /// Set the status
    pub fn status(&mut self, status: impl conversions::IntoStatusCode) -> &mut Self {
        self.response.status = status.into_status_code();
        self
    }

    /// Set the headers
    pub fn headers(&mut self, headers: impl conversions::IntoHeaders) -> &mut Self {
        self.response.headers = into_header_rep(headers.into_headers());
        self
    }

    /// Set a header
    pub fn header(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.response
            .headers
            .insert(key.into().to_lowercase(), HeaderValue::String(value.into()));
        self
    }

    /// Set the body
    pub fn body(&mut self, body: impl conversions::IntoBody) -> &mut Self {
        self.response.body = body.into_body();
        self
    }

    /// Build the `Response`
    pub fn build(&mut self) -> Response {
        std::mem::replace(&mut self.response, Response::new(200, Vec::new()))
    }
}

fn into_header_rep(headers: impl conversions::IntoHeaders) -> HashMap<String, HeaderValue> {
    headers
        .into_headers()
        .into_iter()
        .map(|(k, v)| {
            let v = String::from_utf8(v)
                .map(HeaderValue::String)
                .unwrap_or_else(|e| HeaderValue::Bytes(e.into_bytes()));
            (k.to_lowercase(), v)
        })
        .collect()
}

impl std::hash::Hash for Method {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
    }
}

impl Eq for Method {}

impl PartialEq for Method {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Other(l), Self::Other(r)) => l == r,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl std::fmt::Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Delete => "DELETE",
            Method::Patch => "PATCH",
            Method::Head => "HEAD",
            Method::Options => "OPTIONS",
            Method::Connect => "CONNECT",
            Method::Trace => "TRACE",
            Method::Other(o) => o,
        })
    }
}

impl IncomingRequest {
    /// The incoming request Uri
    pub fn uri(&self) -> String {
        let scheme_and_authority =
            if let (Some(scheme), Some(authority)) = (self.scheme(), self.authority()) {
                let scheme = match &scheme {
                    Scheme::Http => "http://",
                    Scheme::Https => "https://",
                    Scheme::Other(s) => s.as_str(),
                };
                format!("{scheme}{authority}")
            } else {
                String::new()
            };
        let path_and_query = self.path_with_query().unwrap_or_default();
        format!("{scheme_and_authority}{path_and_query}")
    }

    /// Return a `Stream` from which the body of the specified request may be read.
    ///
    /// # Panics
    ///
    /// Panics if the body was already consumed.
    pub fn into_body_stream(self) -> impl futures::Stream<Item = anyhow::Result<Vec<u8>>> {
        executor::incoming_body(self.consume().expect("request body was already consumed"))
    }

    /// Return a `Vec<u8>` of the body or fails
    pub async fn into_body(self) -> anyhow::Result<Vec<u8>> {
        use futures::TryStreamExt;
        let mut stream = self.into_body_stream();
        let mut body = Vec::new();
        while let Some(chunk) = stream.try_next().await? {
            body.extend(chunk);
        }
        Ok(body)
    }
}

impl IncomingResponse {
    /// Return a `Stream` from which the body of the specified response may be read.
    ///
    /// # Panics
    ///
    /// Panics if the body was already consumed.
    pub fn into_body_stream(self) -> impl futures::Stream<Item = anyhow::Result<Vec<u8>>> {
        executor::incoming_body(self.consume().expect("response body was already consumed"))
    }

    /// Return a `Vec<u8>` of the body or fails
    pub async fn into_body(self) -> anyhow::Result<Vec<u8>> {
        use futures::TryStreamExt;
        let mut stream = self.into_body_stream();
        let mut body = Vec::new();
        while let Some(chunk) = stream.try_next().await? {
            body.extend(chunk);
        }
        Ok(body)
    }
}

impl OutgoingResponse {
    /// Construct a `Sink` which writes chunks to the body of the specified response.
    ///
    /// # Panics
    ///
    /// Panics if the body was already taken.
    pub fn take_body(&self) -> impl futures::Sink<Vec<u8>, Error = anyhow::Error> {
        executor::outgoing_body(self.write().expect("response body was already taken"))
    }
}

impl ResponseOutparam {
    /// Set with the outgoing response and the supplied buffer
    ///
    /// Will panic if response body has already been taken
    pub async fn set_with_body(
        self,
        response: OutgoingResponse,
        buffer: Vec<u8>,
    ) -> anyhow::Result<()> {
        use futures::SinkExt;
        let mut body = response.take_body();
        ResponseOutparam::set(self, Ok(response));
        body.send(buffer).await
    }
}

/// Send an outgoing request
pub async fn send<I, O>(request: I) -> Result<O, SendError>
where
    I: TryInto<OutgoingRequest>,
    I::Error: Into<Box<dyn std::error::Error + Send + Sync>> + 'static,
    O: TryFromIncomingResponse,
    O::Error: Into<Box<dyn std::error::Error + Send + Sync>> + 'static,
{
    let response = executor::outgoing_request_send(
        request
            .try_into()
            .map_err(|e| SendError::RequestConversion(e.into()))?,
    )
    .await
    .map_err(SendError::Http)?;
    TryFromIncomingResponse::try_from_incoming_response(response)
        .await
        .map_err(|e: O::Error| SendError::ResponseConversion(e.into()))
}

/// An error encountered when performing an HTTP request
#[derive(thiserror::Error, Debug)]
pub enum SendError {
    /// Error converting to a request
    #[error(transparent)]
    RequestConversion(Box<dyn std::error::Error + Send + Sync>),
    /// Error converting from a response
    #[error(transparent)]
    ResponseConversion(Box<dyn std::error::Error + Send + Sync>),
    /// An HTTP error
    #[error(transparent)]
    Http(Error),
}

#[doc(hidden)]
/// The executor for driving wasi-http futures to completion
mod executor;
#[doc(hidden)]
pub use executor::run;

/// An error parsing a JSON body
#[cfg(feature = "json")]
#[derive(Debug)]
pub struct JsonBodyError(serde_json::Error);

#[cfg(feature = "json")]
impl std::error::Error for JsonBodyError {}

#[cfg(feature = "json")]
impl std::fmt::Display for JsonBodyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("could not convert body to json")
    }
}

/// An error when the body is not UTF-8
#[derive(Debug)]
pub struct NonUtf8BodyError;

impl std::error::Error for NonUtf8BodyError {}

impl std::fmt::Display for NonUtf8BodyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("body was expected to be utf8 but was not")
    }
}

mod router;
/// Exports HTTP Router items.
pub use router::*;

/// A Body extractor
#[derive(Debug)]
pub struct Body<T>(pub T);

impl<T> std::ops::Deref for Body<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A Json extractor
#[derive(Debug)]
pub struct Json<T>(pub T);

impl<T> std::ops::Deref for Json<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Helper functions for creating responses
pub mod responses {
    use super::Response;

    /// Helper function to return a 404 Not Found response.
    pub fn not_found() -> Response {
        Response::new(404, "Not Found")
    }

    /// Helper function to return a 500 Internal Server Error response.
    pub fn internal_server_error() -> Response {
        Response::new(500, "Internal Server Error")
    }

    /// Helper function to return a 405 Method Not Allowed response.
    pub fn method_not_allowed() -> Response {
        Response::new(405, "Method Not Allowed")
    }

    pub(crate) fn bad_request(msg: Option<String>) -> Response {
        Response::new(400, msg.map(|m| m.into_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_uri_parses() {
        let uri = "/hello?world=1";
        let req = Request::new(Method::Get, uri);
        assert_eq!(req.uri(), uri);
        assert_eq!(req.path(), "/hello");
        assert_eq!(req.query(), "world=1");

        let uri = "http://localhost:3000/hello?world=1";
        let req = Request::new(Method::Get, uri);
        assert_eq!(req.uri(), uri);
        assert_eq!(req.path(), "/hello");
        assert_eq!(req.query(), "world=1");

        let uri = "localhost:3000/hello?world=1";
        let req = Request::new(Method::Get, uri);
        assert_eq!(req.uri(), uri);
        assert_eq!(req.path(), "/hello");
        assert_eq!(req.query(), "world=1");
    }
}
