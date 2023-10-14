use futures::{Sink, Stream};

#[doc(inline)]
pub use super::wit::wasi::http::types::*;

impl IncomingRequest {
    /// Return a `Stream` from which the body of the specified request may be read.
    pub fn into_body(self) -> impl Stream<Item = anyhow::Result<Vec<u8>>> {
        executor::incoming_body(self.consume().expect("request should be consumable"))
    }
}

impl IncomingResponse {
    /// Return a `Stream` from which the body of the specified response may be read.
    pub fn into_body(self) -> impl Stream<Item = anyhow::Result<Vec<u8>>> {
        executor::incoming_body(self.consume().expect("response should be consumable"))
    }
}

impl OutgoingResponse {
    /// Construct a `Sink` which writes chunks to the body of the specified response.
    pub fn take_body(&self) -> impl Sink<Vec<u8>, Error = anyhow::Error> {
        executor::outgoing_body(self.write().expect("response should be writable"))
    }
}

/// Send an outgoing request
pub async fn send(request: OutgoingRequest) -> Result<IncomingResponse, Error> {
    executor::outgoing_request_send(request).await
}

/// The executor for driving wasi-http futures to completion
mod executor;

#[doc(inline)]
pub use executor::run;
