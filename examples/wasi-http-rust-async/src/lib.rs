wit_bindgen::generate!({
    world: "wasi:http/proxy",
    path: "wit",
    exports: {
        "wasi:http/incoming-handler": Component
    }
});

use {
    self::{
        exports::wasi::http::incoming_handler::Guest as IncomingHandler,
        wasi::{
            http::{
                outgoing_handler,
                types::{
                    self, Fields, IncomingBody, IncomingRequest, IncomingResponse, Method,
                    OutgoingBody, OutgoingRequest, OutgoingResponse, ResponseOutparam, Scheme,
                },
            },
            io::streams::StreamError,
        },
    },
    anyhow::{anyhow, bail, Error, Result},
    futures::{future, sink, stream, FutureExt, Sink, SinkExt, Stream, StreamExt, TryStreamExt},
    sha2::{Digest, Sha256},
    std::{cell::RefCell, future::Future, rc::Rc, str, task::Poll},
    url::Url,
    wakers::Wakers,
};

mod wakers;

const READ_SIZE: u64 = 16 * 1024;
const MAX_CONCURRENCY: usize = 16;

struct Component;

impl IncomingHandler for Component {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        let wakers = Wakers::default();
        let future = handle_async(wakers.clone(), request, response_out);
        futures::pin_mut!(future);
        wakers.run(future).unwrap();
    }
}

async fn handle_async(
    wakers: Wakers,
    request: IncomingRequest,
    response_out: ResponseOutparam,
) -> Result<()> {
    let method = request.method();
    let path = request.path_with_query();
    let headers = request.headers().entries();

    match (method, path.as_deref()) {
        (Method::Get, Some("/hash-all")) => {
            let urls = headers.iter().filter_map(|(k, v)| {
                (k == "url")
                    .then_some(v)
                    .and_then(|v| str::from_utf8(v).ok())
                    .and_then(|v| Url::parse(v).ok())
            });

            let results = urls.map({
                let wakers = wakers.clone();
                move |url| hash(wakers.clone(), url.clone()).map(move |result| (url, result))
            });

            let mut results = stream::iter(results).buffer_unordered(MAX_CONCURRENCY);

            let response = OutgoingResponse::new(
                200,
                Fields::new(&[("content-type".to_string(), b"text/plain".to_vec())]),
            );

            let mut sink = outgoing_response_body(wakers, &response);

            ResponseOutparam::set(response_out, Ok(response));

            while let Some((url, result)) = results.next().await {
                sink.send(Some(
                    match result {
                        Ok(hash) => format!("{url}: {hash}\n"),
                        Err(e) => format!("{url}: {e:?}\n"),
                    }
                    .into_bytes(),
                ))
                .await?;
            }

            sink.send(None).await?;
        }

        (Method::Post, Some("/echo")) => {
            let response = OutgoingResponse::new(
                200,
                Fields::new(
                    &headers
                        .iter()
                        .filter_map(|(k, v)| {
                            (k == "content-type").then_some((k.clone(), v.clone()))
                        })
                        .collect::<Vec<_>>(),
                ),
            );

            let mut sink = outgoing_response_body(wakers.clone(), &response);

            ResponseOutparam::set(response_out, Ok(response));

            let mut stream = incoming_request_body(wakers, request);
            while let Some(chunk) = stream.try_next().await? {
                sink.send(Some(chunk)).await?;
            }

            sink.send(None).await?;
        }

        _ => {
            let response = OutgoingResponse::new(405, Fields::new(&[]));

            let body = response.write().expect("response should be writable");

            ResponseOutparam::set(response_out, Ok(response));

            OutgoingBody::finish(body, None);
        }
    }

    Ok(())
}

async fn hash(wakers: Wakers, url: Url) -> Result<String> {
    let request = OutgoingRequest::new(
        &Method::Get,
        Some(url.path()),
        Some(&match url.scheme() {
            "http" => Scheme::Http,
            "https" => Scheme::Https,
            scheme => Scheme::Other(scheme.into()),
        }),
        Some(url.authority()),
        Fields::new(&[]),
    );

    let response = outgoing_request_send(wakers.clone(), request).await?;

    let status = response.status();

    if !(200..300).contains(&status) {
        bail!("unexpected status: {status}");
    }

    let mut body = incoming_response_body(wakers, response);

    let mut hasher = Sha256::new();
    while let Some(chunk) = body.try_next().await? {
        hasher.update(&chunk);
    }

    Ok(hex::encode(hasher.finalize()))
}

fn outgoing_response_body(
    wakers: Wakers,
    response: &OutgoingResponse,
) -> impl Sink<Option<Vec<u8>>, Error = Error> {
    outgoing_body(
        wakers,
        response.write().expect("response should be writable"),
    )
}

fn outgoing_body(wakers: Wakers, body: OutgoingBody) -> impl Sink<Option<Vec<u8>>, Error = Error> {
    let stream = body.write().expect("response body should be writable");
    let pair = Rc::new(RefCell::new(Some((stream, body))));

    sink::unfold((), {
        move |(), chunk: Option<Vec<u8>>| {
            future::poll_fn({
                let mut offset = 0;
                let wakers = wakers.clone();
                let pair = pair.clone();

                move |context| {
                    if let Some(chunk) = chunk.as_ref() {
                        let pair = pair.borrow();
                        let (stream, _) = &pair.as_ref().unwrap();

                        loop {
                            assert!(!chunk[offset..].is_empty());

                            match stream.check_write() {
                                Ok(0) => {
                                    wakers.insert(stream.subscribe(), context.waker().clone());
                                    break Poll::Pending;
                                }
                                Ok(count) => {
                                    let count =
                                        usize::try_from(count).unwrap().min(chunk.len() - offset);

                                    match stream.write(&chunk[offset..][..count]) {
                                        Ok(()) => {
                                            // TODO: only flush at end-of-stream
                                            stream.flush().unwrap();
                                            offset += count;
                                            if offset == chunk.len() {
                                                break Poll::Ready(Ok(()));
                                            }
                                        }
                                        Err(_) => break Poll::Ready(Err(anyhow!("I/O error"))),
                                    }
                                }
                                Err(_) => break Poll::Ready(Err(anyhow!("I/O error"))),
                            }
                        }
                    } else {
                        if let Some((stream, body)) = pair.borrow_mut().take() {
                            drop(stream);
                            OutgoingBody::finish(body, None);
                        }
                        Poll::Ready(Ok(()))
                    }
                }
            })
        }
    })
}

fn outgoing_request_send(
    wakers: Wakers,
    request: OutgoingRequest,
) -> impl Future<Output = Result<IncomingResponse, types::Error>> {
    future::poll_fn({
        let response = outgoing_handler::handle(request, None);

        move |context| match &response {
            Ok(response) => {
                if let Some(response) = response.get() {
                    Poll::Ready(response.unwrap())
                } else {
                    wakers.insert(response.subscribe(), context.waker().clone());
                    Poll::Pending
                }
            }
            Err(error) => Poll::Ready(Err(error.clone())),
        }
    })
}

fn incoming_request_body(
    wakers: Wakers,
    request: IncomingRequest,
) -> impl Stream<Item = Result<Vec<u8>>> {
    incoming_body(
        wakers,
        request.consume().expect("request should be consumable"),
    )
}

fn incoming_response_body(
    wakers: Wakers,
    response: IncomingResponse,
) -> impl Stream<Item = Result<Vec<u8>>> {
    incoming_body(
        wakers,
        response.consume().expect("response should be consumable"),
    )
}

fn incoming_body(wakers: Wakers, body: IncomingBody) -> impl Stream<Item = Result<Vec<u8>>> {
    stream::poll_fn({
        let stream = body.stream().expect("response body should be readable");
        let mut pair = Some((stream, body));

        move |context| {
            let result = if let Some((stream, _)) = &pair {
                match stream.read(READ_SIZE) {
                    Ok(buffer) => {
                        if buffer.is_empty() {
                            wakers.insert(stream.subscribe(), context.waker().clone());
                            Poll::Pending
                        } else {
                            Poll::Ready(Some(Ok(buffer)))
                        }
                    }
                    Err(StreamError::Closed) => Poll::Ready(None),
                    Err(StreamError::LastOperationFailed) => {
                        Poll::Ready(Some(Err(anyhow!("I/O error"))))
                    }
                }
            } else {
                Poll::Ready(None)
            };

            if let Poll::Ready(None) = &result {
                if let Some((stream, body)) = pair.take() {
                    drop(stream);
                    IncomingBody::finish(body);
                }
            }

            result
        }
    })
}
