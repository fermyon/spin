wit_bindgen::generate!("proxy" in "../../wit/preview2");

use {
    self::http::{Http, IncomingRequest, ResponseOutparam},
    anyhow::{anyhow, bail, Error, Result},
    cooked_waker::{IntoWaker, WakeRef},
    default_outgoing_http2 as default_outgoing_http,
    futures::{future, sink, stream, FutureExt, Sink, SinkExt, Stream, StreamExt, TryStreamExt},
    poll2 as poll,
    sha2::{Digest, Sha256},
    std::{
        cell::RefCell,
        collections::HashMap,
        error, fmt,
        future::Future,
        mem,
        pin::Pin,
        rc::Rc,
        str,
        sync::Arc,
        task::{Context, Poll, Waker},
    },
    streams2 as streams,
    types2::{self as types, Method, Scheme},
    url::Url,
};

const READ_SIZE: u64 = 16 * 1024;
const MAX_CONCURRENCY: usize = 16;

struct Component;

impl Http for Component {
    fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
        let wakers = Rc::new(RefCell::new(HashMap::new()));
        let future = handle_async(wakers.clone(), request, response_out);
        futures::pin_mut!(future);
        run(wakers, future).unwrap();
    }
}

export_proxy!(Component);

async fn handle_async(
    wakers: Rc<RefCell<HashMap<poll::Pollable, Vec<Waker>>>>,
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

            let response = types::new_outgoing_response(
                200,
                types::new_fields(&[("content-type", b"text/plain")]),
            );

            types::set_response_outparam(response_out, Ok(response))
                .expect("response outparam should be settable");

            let body =
                types::outgoing_response_write(response).expect("response should be writable");

            let mut sink = output_stream_sink(wakers, body);

            let mut results = stream::iter(results).buffer_unordered(MAX_CONCURRENCY);

            while let Some((url, result)) = results.next().await {
                sink.send(
                    match result {
                        Ok(hash) => format!("{url}: {hash}\n"),
                        Err(e) => format!("{url}: {e:?}\n"),
                    }
                    .into_bytes(),
                )
                .await?;
            }

            types::finish_outgoing_stream(body, None);
        }

        _ => {
            let response = types::new_outgoing_response(405, types::new_fields(&[]));

            types::set_response_outparam(response_out, Ok(response))
                .expect("response outparam should be settable");

            types::finish_outgoing_stream(
                types::outgoing_response_write(response).expect("response should be writable"),
                None,
            );
        }
    }

    Ok(())
}

async fn hash(
    wakers: Rc<RefCell<HashMap<poll::Pollable, Vec<Waker>>>>,
    url: Url,
) -> Result<String> {
    let request = types::new_outgoing_request(
        &Method::Get,
        Some(url.path()),
        Some(&match url.scheme() {
            "http" => Scheme::Http,
            "https" => Scheme::Https,
            scheme => Scheme::Other(scheme.into()),
        }),
        url.host().map(|host| host.to_string()).as_deref(),
        types::new_fields(&[]),
    );

    let response = outgoing_request_send(wakers.clone(), request, url.clone()).await?;

    let status = types::incoming_response_status(response);

    if !(200..300).contains(&status) {
        bail!("unexpected status: {status}");
    }

    let mut body = incoming_response_body(wakers, response, url);

    let mut hasher = Sha256::new();
    while let Some(chunk) = body.try_next().await? {
        hasher.update(&chunk);
    }

    Ok(hex::encode(hasher.finalize()))
}

fn output_stream_sink(
    wakers: Rc<RefCell<HashMap<poll::Pollable, Vec<Waker>>>>,
    stream: streams::OutputStream,
) -> impl Sink<Vec<u8>, Error = Error> {
    sink::unfold((), {
        let pollable = streams::subscribe_to_output_stream(stream);

        move |(), chunk: Vec<u8>| {
            future::poll_fn({
                let mut offset = 0;
                let wakers = wakers.clone();

                move |context| {
                    assert!(!chunk[offset..].is_empty());

                    match streams::write(stream, &chunk[offset..]) {
                        Ok(count) => {
                            let count = usize::try_from(count).unwrap();
                            offset += count;
                            if offset == chunk.len() {
                                Poll::Ready(Ok(()))
                            } else {
                                wakers
                                    .borrow_mut()
                                    .entry(pollable)
                                    .or_default()
                                    .push(context.waker().clone());
                                Poll::Pending
                            }
                        }
                        Err(_) => Poll::Ready(Err(anyhow!("I/O error"))),
                    }
                }
            })
        }
    })
}

fn outgoing_request_send(
    wakers: Rc<RefCell<HashMap<poll::Pollable, Vec<Waker>>>>,
    request: types::OutgoingRequest,
    url: Url,
) -> impl Future<Output = Result<types::IncomingResponse, types::Error>> {
    future::poll_fn({
        let response = default_outgoing_http::handle(request, None);
        let pollable = types::listen_to_future_incoming_response(response);

        move |context| {
            if let Some(response) = types::future_incoming_response_get(response) {
                println!("{url} {pollable} ready");
                Poll::Ready(response)
            } else {
                wakers
                    .borrow_mut()
                    .entry(pollable)
                    .or_default()
                    .push(context.waker().clone());
                Poll::Pending
            }
        }
    })
}

fn incoming_response_body(
    wakers: Rc<RefCell<HashMap<poll::Pollable, Vec<Waker>>>>,
    response: types::IncomingResponse,
    url: Url,
) -> impl Stream<Item = Result<Vec<u8>>> {
    stream::poll_fn({
        let body =
            types::incoming_response_consume(response).expect("response should be consumable");
        let pollable = streams::subscribe_to_input_stream(body);
        let mut saw_end = false;

        move |context| {
            if saw_end {
                println!("{url} got end");
                Poll::Ready(None)
            } else {
                match streams::read(body, READ_SIZE) {
                    Ok((buffer, end)) => {
                        if end {
                            types::finish_incoming_stream(body);
                            saw_end = true;
                        }

                        if buffer.is_empty() {
                            if end {
                                println!("{url} got end");
                                Poll::Ready(None)
                            } else {
                                wakers
                                    .borrow_mut()
                                    .entry(pollable)
                                    .or_default()
                                    .push(context.waker().clone());
                                Poll::Pending
                            }
                        } else {
                            println!("{url} got chunk");
                            Poll::Ready(Some(Ok(buffer)))
                        }
                    }
                    Err(_) => Poll::Ready(Some(Err(anyhow!("I/O error")))),
                }
            }
        }
    })
}

fn run(
    wakers: Rc<RefCell<HashMap<poll::Pollable, Vec<Waker>>>>,
    mut future: Pin<&mut impl Future<Output = Result<()>>>,
) -> Result<()> {
    struct DummyWaker;

    impl WakeRef for DummyWaker {
        fn wake_by_ref(&self) {}
    }

    let waker = Arc::new(DummyWaker).into_waker();

    loop {
        match future.as_mut().poll(&mut Context::from_waker(&waker)) {
            Poll::Pending => {
                assert!(!wakers.borrow().is_empty());

                let mut new_wakers = HashMap::new();

                {
                    let (pollables, wakers) = mem::take::<HashMap<_, _>>(&mut wakers.borrow_mut())
                        .into_iter()
                        .unzip::<_, _, Vec<_>, Vec<_>>();

                    for ((ready, pollable), wakers) in poll::poll_oneoff(&pollables)
                        .into_iter()
                        .zip(&pollables)
                        .zip(wakers)
                    {
                        if ready != 0 {
                            for waker in wakers {
                                waker.wake();
                            }
                        } else {
                            new_wakers.insert(*pollable, wakers);
                        }
                    }
                }

                *wakers.borrow_mut() = new_wakers;
            }
            Poll::Ready(result) => break result,
        }
    }
}

impl fmt::Display for types::Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUrl(s) => write!(f, "InvalidUrl: {s}"),
            Self::TimeoutError(s) => write!(f, "TimeoutError: {s}"),
            Self::ProtocolError(s) => write!(f, "ProtocolError: {s}"),
            Self::UnexpectedError(s) => write!(f, "UnexpectedError: {s}"),
        }
    }
}

impl error::Error for types::Error {}
