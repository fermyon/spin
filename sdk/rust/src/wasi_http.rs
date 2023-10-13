pub use super::wit::wasi::http::types;

/// TOOD
pub mod executor {
    use {
        super::super::wit::wasi::{
            http::outgoing_handler,
            http::types::{
                self, IncomingBody, IncomingRequest, IncomingResponse, OutgoingBody,
                OutgoingRequest, OutgoingResponse,
            },
            io::{
                poll,
                streams::{InputStream, OutputStream, StreamError},
            },
        },
        anyhow::{anyhow, Error, Result},
        futures::{future, sink, stream, Sink, Stream},
        std::{
            cell::RefCell,
            future::Future,
            mem,
            pin::Pin,
            rc::Rc,
            sync::{Arc, Mutex},
            task::{Context, Poll, Wake, Waker},
        },
    };

    const READ_SIZE: u64 = 16 * 1024;

    static WAKERS: Mutex<Vec<(poll::Pollable, Waker)>> = Mutex::new(Vec::new());

    /// Run the specified future on an executor based on `wasi::io/poll/poll-list`, blocking until it
    /// yields a result.
    pub fn run<T>(mut future: Pin<&mut impl Future<Output = T>>) -> T {
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

                    for index in poll::poll_list(&pollables) {
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

    /// Construct a `Sink` which writes chunks to the body of the specified response.
    pub fn outgoing_response_body(
        response: &OutgoingResponse,
    ) -> impl Sink<Vec<u8>, Error = Error> {
        outgoing_body(response.write().expect("response should be writable"))
    }

    fn outgoing_body(body: OutgoingBody) -> impl Sink<Vec<u8>, Error = Error> {
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
                                            stream.flush().expect("stream should be flushable");
                                            flushing = true;
                                        }
                                    } else {
                                        let count = usize::try_from(count)
                                            .unwrap()
                                            .min(chunk.len() - offset);

                                        match stream.write(&chunk[offset..][..count]) {
                                            Ok(()) => {
                                                offset += count;
                                            }
                                            Err(_) => break Poll::Ready(Err(anyhow!("I/O error"))),
                                        }
                                    }
                                }
                                Err(_) => break Poll::Ready(Err(anyhow!("I/O error"))),
                            }
                        }
                    }
                })
            }
        })
    }

    /// Send the specified request and return the response.
    pub fn outgoing_request_send(
        request: OutgoingRequest,
    ) -> impl Future<Output = Result<IncomingResponse, types::Error>> {
        future::poll_fn({
            let response = outgoing_handler::handle(request, None);

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

    /// Return a `Stream` from which the body of the specified request may be read.
    pub fn incoming_request_body(request: IncomingRequest) -> impl Stream<Item = Result<Vec<u8>>> {
        incoming_body(request.consume().expect("request should be consumable"))
    }

    /// Return a `Stream` from which the body of the specified response may be read.
    pub fn incoming_response_body(
        response: IncomingResponse,
    ) -> impl Stream<Item = Result<Vec<u8>>> {
        incoming_body(response.consume().expect("response should be consumable"))
    }

    fn incoming_body(body: IncomingBody) -> impl Stream<Item = Result<Vec<u8>>> {
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
                        Err(StreamError::LastOperationFailed(error)) => {
                            Poll::Ready(Some(Err(anyhow!("{}", error.to_debug_string()))))
                        }
                    }
                } else {
                    Poll::Ready(None)
                }
            }
        })
    }
}
