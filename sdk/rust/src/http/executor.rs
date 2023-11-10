use crate::wit::wasi::http::outgoing_handler;
use crate::wit::wasi::http::types::{
    self, IncomingBody, IncomingResponse, OutgoingBody, OutgoingRequest,
};
use crate::wit::wasi::io;
use crate::wit::wasi::io::streams::{InputStream, OutputStream, StreamError};

use futures::{future, sink, stream, Sink, Stream};

use std::cell::RefCell;
use std::future::Future;
use std::mem;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};

const READ_SIZE: u64 = 16 * 1024;

static WAKERS: Mutex<Vec<(io::poll::Pollable, Waker)>> = Mutex::new(Vec::new());

/// Run the specified future to completion blocking until it yields a result.
///
/// Based on an executor using `wasi::io/poll/poll-list`,
pub fn run<T>(future: impl Future<Output = T>) -> T {
    futures::pin_mut!(future);
    struct DummyWaker;

    impl Wake for DummyWaker {
        fn wake(self: Arc<Self>) {}
    }

    let waker = Arc::new(DummyWaker).into();

    loop {
        match future.as_mut().poll(&mut Context::from_waker(&waker)) {
            Poll::Pending => {
                let mut new_wakers = Vec::new();

                let wakers = mem::take::<Vec<_>>(&mut WAKERS.lock().unwrap());

                assert!(!wakers.is_empty());

                let pollables = wakers
                    .iter()
                    .map(|(pollable, _)| pollable)
                    .collect::<Vec<_>>();

                let mut ready = vec![false; wakers.len()];

                for index in io::poll::poll_list(&pollables) {
                    ready[usize::try_from(index).unwrap()] = true;
                }

                for (ready, (pollable, waker)) in ready.into_iter().zip(wakers) {
                    if ready {
                        waker.wake()
                    } else {
                        new_wakers.push((pollable, waker));
                    }
                }

                *WAKERS.lock().unwrap() = new_wakers;
            }
            Poll::Ready(result) => break result,
        }
    }
}

pub(crate) fn outgoing_body(body: OutgoingBody) -> impl Sink<Vec<u8>, Error = types::Error> {
    struct Outgoing(Option<(OutputStream, OutgoingBody)>);

    impl Drop for Outgoing {
        fn drop(&mut self) {
            if let Some((stream, body)) = self.0.take() {
                drop(stream);
                OutgoingBody::finish(body, None);
            }
        }
    }

    let stream = body.write().expect("response body should be writable");
    let pair = Rc::new(RefCell::new(Outgoing(Some((stream, body)))));

    sink::unfold((), {
        move |(), chunk: Vec<u8>| {
            future::poll_fn({
                let mut offset = 0;
                let mut flushing = false;
                let pair = pair.clone();

                move |context| {
                    let pair = pair.borrow();
                    let (stream, _) = &pair.0.as_ref().unwrap();
                    loop {
                        match stream.check_write() {
                            Ok(0) => {
                                WAKERS
                                    .lock()
                                    .unwrap()
                                    .push((stream.subscribe(), context.waker().clone()));

                                break Poll::Pending;
                            }
                            Ok(count) => {
                                if offset == chunk.len() {
                                    if flushing {
                                        break Poll::Ready(Ok(()));
                                    } else {
                                        match stream.flush() {
                                            Ok(()) => flushing = true,
                                            Err(StreamError::Closed) => break Poll::Ready(Ok(())),
                                            Err(StreamError::LastOperationFailed(e)) => {
                                                break Poll::Ready(Err(
                                                    types::Error::ProtocolError(format!(
                                                        "I/O error: {e}"
                                                    )),
                                                ))
                                            }
                                        }
                                    }
                                } else {
                                    let count =
                                        usize::try_from(count).unwrap().min(chunk.len() - offset);

                                    match stream.write(&chunk[offset..][..count]) {
                                        Ok(()) => {
                                            offset += count;
                                        }
                                        Err(e) => {
                                            break Poll::Ready(Err(types::Error::ProtocolError(
                                                format!("I/O error: {e}"),
                                            )))
                                        }
                                    }
                                }
                            }
                            // If the stream is closed but the entire chunk was
                            // written then we've done all we could so this
                            // chunk is now complete.
                            Err(StreamError::Closed) if offset == chunk.len() => {
                                break Poll::Ready(Ok(()))
                            }
                            Err(e) => {
                                break Poll::Ready(Err(types::Error::ProtocolError(format!(
                                    "I/O error: {e}"
                                ))))
                            }
                        }
                    }
                }
            })
        }
    })
}

/// Send the specified request and return the response.
pub(crate) fn outgoing_request_send(
    request: OutgoingRequest,
) -> impl Future<Output = Result<IncomingResponse, types::Error>> {
    let response = outgoing_handler::handle(request, None);
    future::poll_fn({
        move |context| match &response {
            Ok(response) => {
                if let Some(response) = response.get() {
                    Poll::Ready(response.unwrap())
                } else {
                    WAKERS
                        .lock()
                        .unwrap()
                        .push((response.subscribe(), context.waker().clone()));
                    Poll::Pending
                }
            }
            Err(error) => Poll::Ready(Err(error.clone())),
        }
    })
}

#[doc(hidden)]
pub fn incoming_body(
    body: IncomingBody,
) -> impl Stream<Item = Result<Vec<u8>, io::streams::Error>> {
    struct Incoming(Option<(InputStream, IncomingBody)>);

    impl Drop for Incoming {
        fn drop(&mut self) {
            if let Some((stream, body)) = self.0.take() {
                drop(stream);
                IncomingBody::finish(body);
            }
        }
    }

    stream::poll_fn({
        let stream = body.stream().expect("response body should be readable");
        let pair = Incoming(Some((stream, body)));

        move |context| {
            if let Some((stream, _)) = &pair.0 {
                match stream.read(READ_SIZE) {
                    Ok(buffer) => {
                        if buffer.is_empty() {
                            WAKERS
                                .lock()
                                .unwrap()
                                .push((stream.subscribe(), context.waker().clone()));
                            Poll::Pending
                        } else {
                            Poll::Ready(Some(Ok(buffer)))
                        }
                    }
                    Err(StreamError::Closed) => Poll::Ready(None),
                    Err(StreamError::LastOperationFailed(error)) => Poll::Ready(Some(Err(error))),
                }
            } else {
                Poll::Ready(None)
            }
        }
    })
}
