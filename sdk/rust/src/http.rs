/// Traits for converting between the various types
pub mod conversions;

#[doc(inline)]
pub use conversions::IntoResponse;

use self::conversions::TryFromIncomingResponse;

#[doc(inline)]
pub use super::wit::wasi::http::types::*;

type Request = hyperium::Request<Vec<u8>>;
type Response = hyperium::Response<Vec<u8>>;

impl IncomingRequest {
    /// The incoming request Uri
    pub fn uri(&self) -> Result<hyperium::Uri, hyperium::uri::InvalidUri> {
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
        format!("{scheme_and_authority}{path_and_query}").parse()
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
    /// Helper function to return a 404 Not Found response.
    pub fn not_found() -> hyperium::Response<&'static str> {
        hyperium::Response::builder()
            .status(404)
            .body("Not Found")
            .unwrap()
    }

    /// Helper function to return a 500 Internal Server Error response.
    pub fn internal_server_error() -> hyperium::Response<&'static str> {
        hyperium::Response::builder()
            .status(500)
            .body("Internal Server Error")
            .unwrap()
    }

    /// Helper function to return a 405 Method Not Allowed response.
    pub fn method_not_allowed() -> hyperium::Response<&'static str> {
        hyperium::Response::builder()
            .status(405)
            .body("Method Not Allowed")
            .unwrap()
    }

    pub(crate) fn bad_request(msg: Option<String>) -> hyperium::Response<Option<String>> {
        hyperium::Response::builder().status(400).body(msg).unwrap()
    }
}
