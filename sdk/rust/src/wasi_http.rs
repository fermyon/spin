use futures::{Sink, Stream};

/// Traits for converting between the various types
pub mod conversions;
#[doc(hidden)]
/// The executor for driving wasi-http futures to completion
mod executor;

#[doc(inline)]
pub use super::wit::wasi::http::types::*;

impl IncomingRequest {
    /// Return a `Stream` from which the body of the specified request may be read.
    ///
    /// # Panics
    ///
    /// Panics if the body was already consumed.
    pub fn into_body_stream(self) -> impl Stream<Item = anyhow::Result<Vec<u8>>> {
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
    pub fn into_body_stream(self) -> impl Stream<Item = anyhow::Result<Vec<u8>>> {
        executor::incoming_body(self.consume().expect("response body was already consumed"))
    }
}

impl OutgoingResponse {
    /// Construct a `Sink` which writes chunks to the body of the specified response.
    ///
    /// # Panics
    ///
    /// Panics if the body was already taken.
    pub fn take_body(&self) -> impl Sink<Vec<u8>, Error = anyhow::Error> {
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
    O: TryFrom<IncomingResponse>,
    O::Error: Into<Box<dyn std::error::Error + Send + Sync>> + 'static,
{
    let response = executor::outgoing_request_send(
        request
            .try_into()
            .map_err(|e| SendError::RequestConversion(e.into()))?,
    )
    .await
    .map_err(SendError::Http)?;
    response
        .try_into()
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

#[doc(inline)]
pub use executor::run;
