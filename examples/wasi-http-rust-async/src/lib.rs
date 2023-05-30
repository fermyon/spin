wit_bindgen::generate!("proxy" in "../../wit/wasi-http");

use {
    self::{
        exports::wasi::http::incoming_handler::IncomingHandler,
        wasi::{
            http::{
                outgoing_handler,
                types2::{self as types, IncomingRequest, Method, ResponseOutparam, Scheme},
            },
            io::streams2::{self as streams, StreamStatus},
        },
    },
    anyhow::{anyhow, bail, Error, Result},
    futures::{future, sink, stream, FutureExt, Sink, SinkExt, Stream, StreamExt, TryStreamExt},
    sha2::{Digest, Sha256},
    std::{future::Future, ops::Deref, str, task::Poll},
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

export_proxy!(Component);

async fn handle_async(
    wakers: Wakers,
    request: IncomingRequest,
    response_out: ResponseOutparam,
) -> Result<()> {
    let method = types::incoming_request_method(request);
    let path = types::incoming_request_path_with_query(request);
    let headers = types::fields_entries(types::incoming_request_headers(request));

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

            let response = types::new_outgoing_response(
                200,
                types::new_fields(&[("content-type", b"text/plain")]),
            )?;

            types::set_response_outparam(response_out, Ok(response))
                .expect("response outparam should be settable");

            let mut sink = outgoing_response_body(wakers, response);

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
            let response = types::new_outgoing_response(
                200,
                types::new_fields(
                    &headers
                        .iter()
                        .filter_map(|(k, v)| {
                            (k == "content-type").then_some((k.deref(), v.deref()))
                        })
                        .collect::<Vec<_>>(),
                ),
            )?;

            types::set_response_outparam(response_out, Ok(response))
                .expect("response outparam should be settable");

            let mut stream = incoming_request_body(wakers.clone(), request);
            let mut sink = outgoing_response_body(wakers, response);

            while let Some(chunk) = stream.try_next().await? {
                sink.send(Some(chunk)).await?;
            }

            sink.send(None).await?;
        }

        _ => {
            let response = types::new_outgoing_response(405, types::new_fields(&[]))?;

            types::set_response_outparam(response_out, Ok(response))
                .expect("response outparam should be settable");

            types::finish_outgoing_stream(
                types::outgoing_response_write(response).expect("response should be writable"),
            );
        }
    }

    Ok(())
}

async fn hash(wakers: Wakers, url: Url) -> Result<String> {
    let request = types::new_outgoing_request(
        &Method::Get,
        Some(url.path()),
        Some(&match url.scheme() {
            "http" => Scheme::Http,
            "https" => Scheme::Https,
            scheme => Scheme::Other(scheme.into()),
        }),
        Some(url.authority()),
        types::new_fields(&[]),
    )?;

    let response = outgoing_request_send(wakers.clone(), request).await?;

    let status = types::incoming_response_status(response);

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
    response: types::OutgoingResponse,
) -> impl Sink<Option<Vec<u8>>, Error = Error> {
    outgoing_body(
        wakers,
        types::outgoing_response_write(response).expect("response should be writable"),
    )
}

fn outgoing_body(
    wakers: Wakers,
    body: streams::OutputStream,
) -> impl Sink<Option<Vec<u8>>, Error = Error> {
    sink::unfold((), {
        let pollable = streams::subscribe_to_output_stream(body);

        move |(), chunk: Option<Vec<u8>>| {
            future::poll_fn({
                let mut offset = 0;
                let wakers = wakers.clone();

                move |context| {
                    if let Some(chunk) = chunk.as_ref() {
                        assert!(!chunk[offset..].is_empty());

                        match streams::write(body, &chunk[offset..]) {
                            Ok(count) => {
                                let count = usize::try_from(count).unwrap();
                                offset += count;
                                if offset == chunk.len() {
                                    Poll::Ready(Ok(()))
                                } else {
                                    wakers.insert(pollable, context.waker().clone());
                                    Poll::Pending
                                }
                            }
                            Err(_) => Poll::Ready(Err(anyhow!("I/O error"))),
                        }
                    } else {
                        types::finish_outgoing_stream(body);
                        Poll::Ready(Ok(()))
                    }
                }
            })
        }
    })
}

fn outgoing_request_send(
    wakers: Wakers,
    request: types::OutgoingRequest,
) -> impl Future<Output = Result<types::IncomingResponse, types::Error>> {
    future::poll_fn({
        let response = outgoing_handler::handle(request, None);
        let pollable = types::listen_to_future_incoming_response(response);

        move |context| {
            if let Some(response) = types::future_incoming_response_get(response) {
                Poll::Ready(response)
            } else {
                wakers.insert(pollable, context.waker().clone());
                Poll::Pending
            }
        }
    })
}

fn incoming_request_body(
    wakers: Wakers,
    request: types::IncomingRequest,
) -> impl Stream<Item = Result<Vec<u8>>> {
    incoming_body(
        wakers,
        types::incoming_request_consume(request).expect("request should be consumable"),
    )
}

fn incoming_response_body(
    wakers: Wakers,
    response: types::IncomingResponse,
) -> impl Stream<Item = Result<Vec<u8>>> {
    incoming_body(
        wakers,
        types::incoming_response_consume(response).expect("response should be consumable"),
    )
}

fn incoming_body(wakers: Wakers, body: types::InputStream) -> impl Stream<Item = Result<Vec<u8>>> {
    stream::poll_fn({
        let pollable = streams::subscribe_to_input_stream(body);
        let mut saw_end = false;

        move |context| {
            if saw_end {
                Poll::Ready(None)
            } else {
                match streams::read(body, READ_SIZE) {
                    Ok((buffer, status)) => {
                        if let StreamStatus::Ended = status {
                            types::finish_incoming_stream(body);
                            saw_end = true;
                        }

                        if buffer.is_empty() {
                            if let StreamStatus::Ended = status {
                                Poll::Ready(None)
                            } else {
                                wakers.insert(pollable, context.waker().clone());
                                Poll::Pending
                            }
                        } else {
                            Poll::Ready(Some(Ok(buffer)))
                        }
                    }
                    Err(_) => Poll::Ready(Some(Err(anyhow!("I/O error")))),
                }
            }
        }
    })
}
