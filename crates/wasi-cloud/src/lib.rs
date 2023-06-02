#![allow(unused)] // temporary, until `todo!()`s are filled in

use anyhow::{anyhow, bail, Error, Result};
use cooked_waker::{IntoWaker, WakeRef};
use default_outgoing_http2 as default_outgoing_http;
use futures::{
    channel::{mpsc, oneshot},
    future::{self, BoxFuture},
    Future, FutureExt, Sink, SinkExt, Stream, StreamExt, TryFutureExt, TryStreamExt,
};
use http_crate::header::{HeaderMap, HeaderName, HeaderValue};
use hyper::{
    body::{self, Bytes, HttpBody},
    Body,
};
use poll2 as poll;
use reqwest::Client;
use spin_common::table::Table;
use spin_core::{async_trait, HostComponent};
use std::{
    error, fmt,
    pin::Pin,
    str::FromStr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    task::{Context, Poll},
};
use streams2 as streams;
use tokio::{sync::Notify, task};
use types2::{self as types, Method, Scheme};

wasmtime::component::bindgen!({
    path: "../../wit/preview2",
    world: "proxy",
    async: true
});

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

pub struct WasiCloudComponent;

impl HostComponent for WasiCloudComponent {
    type Data = WasiCloud;

    fn add_to_linker<T: Send>(
        linker: &mut spin_core::Linker<T>,
        get: impl Fn(&mut spin_core::Data<T>) -> &mut Self::Data + Send + Sync + Copy + 'static,
    ) -> anyhow::Result<()> {
        Proxy::add_to_linker(linker, get)
    }

    fn build_data(&self) -> Self::Data {
        Default::default()
    }
}

pub struct IncomingRequest {
    pub method: Method,
    pub path_with_query: Option<String>,
    pub scheme: Option<Scheme>,
    pub authority: Option<String>,
    pub headers: Fields,
    pub body: Option<Body>,
}

type FieldEntries = Vec<(String, Vec<u8>)>;

#[derive(Clone)]
pub struct Fields(pub Arc<Mutex<FieldEntries>>);

#[derive(Clone)]
struct Pollable(Arc<AtomicBool>);

struct PollWaker {
    pollable: Pollable,
    notify: Arc<Notify>,
}

impl WakeRef for PollWaker {
    fn wake_by_ref(&self) {
        self.pollable.0.store(true, Ordering::SeqCst);
        self.notify.notify_one();
    }
}

pub struct ResponseOutparam(
    pub Option<oneshot::Sender<Result<OutgoingResponseReceiver, types::Error>>>,
);

struct OutgoingResponse {
    status: types::StatusCode,
    headers: FieldEntries,
    sender: Option<body::Sender>,
    body: Option<Body>,
}

impl OutgoingResponse {
    fn receiver(&mut self) -> Option<OutgoingResponseReceiver> {
        Some(OutgoingResponseReceiver {
            status: self.status,
            headers: self.headers.clone(),
            body: self.body.take()?,
        })
    }
}

#[derive(Debug)]
pub struct OutgoingResponseReceiver {
    pub status: types::StatusCode,
    pub headers: FieldEntries,
    pub body: Body,
}

struct OutgoingRequest {
    method: Method,
    path_with_query: Option<String>,
    scheme: Option<Scheme>,
    authority: Option<String>,
    headers: FieldEntries,
    sender: Option<mpsc::Sender<Result<Bytes>>>,
    receiver: Option<mpsc::Receiver<Result<Bytes>>>,
}

struct IncomingResponse {
    status: types::StatusCode,
    headers: Fields,
    body: Option<Body>,
}

struct FutureIncomingResponse {
    pollable: Pollable,
    response: BoxFuture<'static, Result<IncomingResponse, types::Error>>,
}

struct InputStream {
    chunk: Option<Bytes>,
    pollable: Pollable,
    body: Body,
    end_of_stream: bool,
    trailers: Option<HeaderMap<HeaderValue>>,
}

impl InputStream {
    async fn read(
        &mut self,
        len: u64,
        notify: Option<Arc<Notify>>,
    ) -> Result<(Vec<u8>, bool), streams::StreamError> {
        let len = usize::try_from(len).map_err(|_| streams::StreamError {})?;

        loop {
            if self.end_of_stream {
                let ended = if let Some(notify) = notify {
                    self.pollable.0.store(false, Ordering::SeqCst);

                    match Pin::new(&mut self.body.trailers()).poll(&mut Context::from_waker(
                        &Arc::new(PollWaker {
                            pollable: self.pollable.clone(),
                            notify,
                        })
                        .into_waker(),
                    )) {
                        Poll::Pending => false,
                        Poll::Ready(trailers) => {
                            self.pollable.0.store(true, Ordering::SeqCst);
                            self.trailers = trailers.map_err(|_| streams::StreamError {})?;
                            true
                        }
                    }
                } else {
                    self.trailers = self
                        .body
                        .trailers()
                        .await
                        .map_err(|_| streams::StreamError {})?;
                    true
                };

                break Ok((Vec::new(), ended));
            } else {
                let (result, chunk) = if let Some(mut chunk) = self.chunk.take() {
                    let remainder = chunk.split_off(len.min(chunk.len()));
                    (
                        Some(chunk.to_vec()),
                        if remainder.is_empty() {
                            None
                        } else {
                            Some(remainder)
                        },
                    )
                } else {
                    (None, None)
                };

                self.chunk = chunk;

                let result = if let Some(result) = result {
                    Some((result, false))
                } else if let Some(notify) = notify.as_ref() {
                    self.pollable.0.store(false, Ordering::SeqCst);

                    match Pin::new(&mut self.body).poll_next(&mut Context::from_waker(
                        &Arc::new(PollWaker {
                            pollable: self.pollable.clone(),
                            notify: notify.clone(),
                        })
                        .into_waker(),
                    )) {
                        Poll::Pending => Some((Vec::new(), false)),
                        Poll::Ready(chunk) => {
                            self.pollable.0.store(true, Ordering::SeqCst);

                            if let Some(chunk) = chunk {
                                let chunk = chunk.map_err(|_| streams::StreamError {})?;
                                self.chunk = if chunk.is_empty() { None } else { Some(chunk) };
                                None
                            } else {
                                self.end_of_stream = true;
                                None
                            }
                        }
                    }
                } else if let Some(chunk) = self
                    .body
                    .try_next()
                    .await
                    .map_err(|_| streams::StreamError {})?
                {
                    self.chunk = if chunk.is_empty() { None } else { Some(chunk) };
                    None
                } else {
                    self.end_of_stream = true;
                    None
                };

                if let Some(result) = result {
                    break Ok(result);
                }
            }
        }
    }
}

enum Sender {
    Hyper(Option<body::Sender>),
    Reqwest(mpsc::Sender<Result<Bytes>>),
}

struct OutputStream {
    pollable: Pollable,
    sender: Sender,
}

impl OutputStream {
    async fn write(
        &mut self,
        buf: Vec<u8>,
        notify: Option<Arc<Notify>>,
    ) -> Result<u64, streams::StreamError> {
        let len = u64::try_from(buf.len()).unwrap();

        let chunk = buf.into();

        if let Sender::Hyper(None) = &self.sender {
            return Err(streams::StreamError {});
        }

        if let Some(notify) = notify {
            self.pollable.0.store(false, Ordering::SeqCst);

            let waker = Arc::new(PollWaker {
                pollable: self.pollable.clone(),
                notify,
            })
            .into_waker();

            let mut context = Context::from_waker(&waker);

            let poll = match &mut self.sender {
                Sender::Hyper(Some(sender)) => {
                    Pin::new(sender).poll_ready(&mut context).map_err(drop)
                }
                Sender::Reqwest(sender) => Pin::new(sender).poll_ready(&mut context).map_err(drop),
                Sender::Hyper(None) => unreachable!(),
            };

            match poll {
                Poll::Pending => Ok(0),
                Poll::Ready(result) => {
                    self.pollable.0.store(true, Ordering::SeqCst);

                    match result {
                        Ok(()) => {
                            let result = match &mut self.sender {
                                Sender::Hyper(Some(sender)) => {
                                    sender.try_send_data(chunk).map_err(drop)
                                }
                                Sender::Reqwest(sender) => {
                                    Pin::new(sender).start_send(Ok(chunk)).map_err(drop)
                                }
                                Sender::Hyper(None) => unreachable!(),
                            };
                            result.expect(
                                "`start_send` should succeed after \
                                 `poll_ready` indicates readiness",
                            );

                            Ok(len)
                        }
                        Err(()) => Err(streams::StreamError {}),
                    }
                }
            }
        } else {
            match &mut self.sender {
                Sender::Hyper(Some(sender)) => sender.send_data(chunk).await.map_err(drop),
                Sender::Reqwest(sender) => sender.send(Ok(chunk)).await.map_err(drop),
                Sender::Hyper(None) => unreachable!(),
            }
            .map(|()| len)
            .map_err(|_| streams::StreamError {})
        }
    }
}

#[derive(Default)]
pub struct WasiCloud {
    incoming_requests: Table<IncomingRequest>,
    outgoing_responses: Table<OutgoingResponse>,
    outgoing_requests: Table<OutgoingRequest>,
    incoming_responses: Table<IncomingResponse>,
    future_incoming_responses: Table<FutureIncomingResponse>,
    fields: Table<Fields>,
    response_outparams: Table<ResponseOutparam>,
    pollables: Table<Pollable>,
    input_streams: Table<InputStream>,
    output_streams: Table<OutputStream>,
    notify: Arc<Notify>,
    client: Client,
}

impl WasiCloud {
    pub fn push_incoming_request(
        &mut self,
        request: IncomingRequest,
    ) -> Result<types::IncomingRequest> {
        self.incoming_requests
            .push(request)
            .map_err(|()| anyhow!("table overflow"))
    }

    pub fn push_response_outparam(
        &mut self,
        outparam: ResponseOutparam,
    ) -> Result<types::ResponseOutparam> {
        self.response_outparams
            .push(outparam)
            .map_err(|()| anyhow!("table overflow"))
    }
}

// #[async_trait]
// impl wall_clock::Host for WasiCloud {
//     async fn now(&mut self) -> Result<wall_clock::Datetime> {
//         todo!()
//     }

//     async fn resolution(&mut self) -> Result<wall_clock::Datetime> {
//         todo!()
//     }
// }

// #[async_trait]
// impl monotonic_clock::Host for WasiCloud {
//     async fn now(&mut self) -> Result<monotonic_clock::Instant> {
//         todo!()
//     }

//     async fn resolution(&mut self) -> Result<monotonic_clock::Instant> {
//         todo!()
//     }

//     async fn subscribe(
//         &mut self,
//         when: monotonic_clock::Instant,
//         absolute: bool,
//     ) -> Result<monotonic_clock::Pollable> {
//         todo!()
//     }
// }

// #[async_trait]
// impl timezone::Host for WasiCloud {
//     async fn display(
//         &mut self,
//         this: timezone::Timezone,
//         when: timezone::Datetime,
//     ) -> Result<timezone::TimezoneDisplay> {
//         todo!()
//     }

//     async fn utc_offset(
//         &mut self,
//         this: timezone::Timezone,
//         when: timezone::Datetime,
//     ) -> Result<i32> {
//         todo!()
//     }

//     async fn drop_timezone(&mut self, this: timezone::Timezone) -> Result<()> {
//         todo!()
//     }
// }

#[async_trait]
impl poll::Host for WasiCloud {
    async fn drop_pollable(&mut self, this: poll::Pollable) -> Result<()> {
        self.pollables.remove(this);
        Ok(())
    }

    async fn poll_oneoff(&mut self, pollables: Vec<poll::Pollable>) -> Result<Vec<u8>> {
        let pollables = pollables
            .iter()
            .map(|handle| {
                self.pollables
                    .get(*handle)
                    .ok_or_else(|| anyhow!("unknown handle: {handle}"))
            })
            .collect::<Result<Vec<_>>>()?;

        loop {
            let mut ready = false;
            let result = pollables
                .iter()
                .map(|pollable| {
                    if pollable.0.load(Ordering::SeqCst) {
                        ready = true;
                        1
                    } else {
                        0
                    }
                })
                .collect();

            if ready {
                break Ok(result);
            } else {
                self.notify.notified().await;
            }
        }
    }
}

// #[async_trait]
// impl random::Host for WasiCloud {
//     async fn get_random_bytes(&mut self, len: u64) -> Result<Vec<u8>> {
//         todo!()
//     }

//     async fn get_random_u64(&mut self) -> Result<u64> {
//         todo!()
//     }
// }

#[async_trait]
impl streams::Host for WasiCloud {
    async fn read(
        &mut self,
        this: streams::InputStream,
        len: u64,
    ) -> Result<Result<(Vec<u8>, bool), streams::StreamError>> {
        Ok(self
            .input_streams
            .get_mut(this)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))?
            .read(len, Some(self.notify.clone()))
            .await)
    }

    async fn blocking_read(
        &mut self,
        this: streams::InputStream,
        len: u64,
    ) -> Result<Result<(Vec<u8>, bool), streams::StreamError>> {
        Ok(self
            .input_streams
            .get_mut(this)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))?
            .read(len, None)
            .await)
    }

    async fn skip(
        &mut self,
        this: streams::InputStream,
        len: u64,
    ) -> Result<Result<(u64, bool), streams::StreamError>> {
        todo!()
    }

    async fn blocking_skip(
        &mut self,
        this: streams::InputStream,
        len: u64,
    ) -> Result<Result<(u64, bool), streams::StreamError>> {
        todo!()
    }

    async fn subscribe_to_input_stream(
        &mut self,
        this: streams::InputStream,
    ) -> Result<streams::Pollable> {
        self.pollables
            .push(
                self.input_streams
                    .get_mut(this)
                    .ok_or_else(|| anyhow!("unknown handle: {this}"))?
                    .pollable
                    .clone(),
            )
            .map_err(|()| anyhow!("table overflow"))
    }

    async fn drop_input_stream(&mut self, this: streams::InputStream) -> Result<()> {
        self.input_streams
            .remove(this)
            .map(drop)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))
    }

    async fn write(
        &mut self,
        this: streams::OutputStream,
        buf: Vec<u8>,
    ) -> Result<Result<u64, streams::StreamError>> {
        Ok(self
            .output_streams
            .get_mut(this)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))?
            .write(buf, Some(self.notify.clone()))
            .await)
    }

    async fn blocking_write(
        &mut self,
        this: streams::OutputStream,
        buf: Vec<u8>,
    ) -> Result<Result<u64, streams::StreamError>> {
        Ok(self
            .output_streams
            .get_mut(this)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))?
            .write(buf, None)
            .await)
    }

    async fn write_zeroes(
        &mut self,
        this: streams::OutputStream,
        len: u64,
    ) -> Result<Result<u64, streams::StreamError>> {
        todo!()
    }

    async fn blocking_write_zeroes(
        &mut self,
        this: streams::OutputStream,
        len: u64,
    ) -> Result<Result<u64, streams::StreamError>> {
        todo!()
    }

    async fn splice(
        &mut self,
        this: streams::OutputStream,
        src: streams::InputStream,
        len: u64,
    ) -> Result<Result<(u64, bool), streams::StreamError>> {
        todo!()
    }

    async fn blocking_splice(
        &mut self,
        this: streams::OutputStream,
        src: streams::InputStream,
        len: u64,
    ) -> Result<Result<(u64, bool), streams::StreamError>> {
        todo!()
    }

    async fn forward(
        &mut self,
        this: streams::OutputStream,
        src: streams::InputStream,
    ) -> Result<Result<u64, streams::StreamError>> {
        todo!()
    }

    async fn subscribe_to_output_stream(
        &mut self,
        this: streams::OutputStream,
    ) -> Result<streams::Pollable> {
        self.pollables
            .push(
                self.output_streams
                    .get_mut(this)
                    .ok_or_else(|| anyhow!("unknown handle: {this}"))?
                    .pollable
                    .clone(),
            )
            .map_err(|()| anyhow!("table overflow"))
    }

    async fn drop_output_stream(&mut self, this: streams::OutputStream) -> Result<()> {
        self.output_streams
            .remove(this)
            .map(drop)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))
    }
}

// #[async_trait]
// impl stdout::Host for WasiCloud {
//     async fn get_stdout(&mut self) -> Result<stdout::OutputStream> {
//         todo!()
//     }
// }

// #[async_trait]
// impl stderr::Host for WasiCloud {
//     async fn get_stderr(&mut self) -> Result<stderr::OutputStream> {
//         todo!()
//     }
// }

// #[async_trait]
// impl stdin::Host for WasiCloud {
//     async fn get_stdin(&mut self) -> Result<stdin::InputStream> {
//         todo!()
//     }
// }

#[async_trait]
impl types::Host for WasiCloud {
    async fn drop_fields(&mut self, this: types::Fields) -> Result<()> {
        self.fields
            .remove(this)
            .map(drop)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))
    }

    async fn new_fields(&mut self, entries: FieldEntries) -> Result<types::Fields> {
        self.fields
            .push(Fields(Arc::new(Mutex::new(entries))))
            .map_err(|()| anyhow!("table overflow"))
    }

    async fn fields_get(&mut self, this: types::Fields, name: String) -> Result<Vec<Vec<u8>>> {
        Ok(self
            .fields
            .get(this)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))?
            .0
            .lock()
            .unwrap()
            .iter()
            .filter_map(|(k, v)| (k == &name).then(|| v.clone()))
            .collect())
    }

    async fn fields_set(
        &mut self,
        this: types::Fields,
        name: String,
        values: Vec<Vec<u8>>,
    ) -> Result<()> {
        let mut vec = self
            .fields
            .get(this)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))?
            .0
            .lock()
            .unwrap();

        vec.retain(|(k, _)| k != &name);

        for value in values {
            vec.push((name.clone(), value));
        }

        Ok(())
    }

    async fn fields_delete(&mut self, this: types::Fields, name: String) -> Result<()> {
        self.fields
            .get(this)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))?
            .0
            .lock()
            .unwrap()
            .retain(|(k, _)| k != &name);

        Ok(())
    }

    async fn fields_append(
        &mut self,
        this: types::Fields,
        name: String,
        value: Vec<u8>,
    ) -> Result<()> {
        let mut vec = self
            .fields
            .get(this)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))?
            .0
            .lock()
            .unwrap()
            .push((name, value));

        Ok(())
    }

    async fn fields_entries(&mut self, this: types::Fields) -> Result<FieldEntries> {
        Ok(self
            .fields
            .get(this)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))?
            .0
            .lock()
            .unwrap()
            .clone())
    }

    async fn fields_clone(&mut self, this: types::Fields) -> Result<types::Fields> {
        let entries = self.fields_entries(this).await?;

        self.fields
            .push(Fields(Arc::new(Mutex::new(entries))))
            .map_err(|()| anyhow!("table overflow"))
    }

    async fn finish_incoming_stream(
        &mut self,
        this: types::IncomingStream,
    ) -> Result<Option<types::Trailers>> {
        let trailers = self
            .input_streams
            .get_mut(this)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))?
            .trailers
            .take();

        trailers
            .map(|trailers| {
                self.fields
                    .push(Fields(Arc::new(Mutex::new(
                        trailers
                            .iter()
                            .map(|(k, v)| (k.to_string(), v.as_bytes().to_vec()))
                            .collect(),
                    ))))
                    .map_err(|()| anyhow!("table overflow"))
            })
            .transpose()
    }

    async fn finish_outgoing_stream(
        &mut self,
        this: types::OutgoingStream,
        trailers: Option<types::Trailers>,
    ) -> Result<()> {
        // TODO: We should change the WIT file so this can return an I/O error instead of trapping

        let trailers = trailers
            .map(|trailers| {
                self.fields
                    .get(trailers)
                    .ok_or_else(|| anyhow!("unknown handle: {trailers}"))?
                    .0
                    .lock()
                    .unwrap()
                    .iter()
                    .map(|(k, v)| Ok((HeaderName::from_str(k)?, HeaderValue::from_bytes(v)?)))
                    .collect::<Result<_>>()
            })
            .transpose()?;

        let sender = &mut self
            .output_streams
            .get_mut(this)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))?
            .sender;

        if let Some(trailers) = trailers {
            match sender {
                Sender::Hyper(sender) => {
                    if let Some(mut sender) = sender.take() {
                        // TODO: this is the only way we can avoid blocking; should we change the WIT to return a
                        // future instead?
                        task::spawn(async move { sender.send_trailers(trailers).await });
                    } else {
                        bail!("stream already finished");
                    }
                }
                // TODO: will probably need to contribute trailer support upstream to `reqwest` or else use `hyper`
                // directly:
                Sender::Reqwest(_) => bail!("trailers not yet supported for outgoing requests"),
            }
        }

        Ok(())
    }

    async fn drop_incoming_request(&mut self, this: types::IncomingRequest) -> Result<()> {
        self.incoming_requests
            .remove(this)
            .map(drop)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))
    }

    async fn drop_outgoing_request(&mut self, this: types::OutgoingRequest) -> Result<()> {
        self.outgoing_requests
            .remove(this)
            .map(drop)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))
    }

    async fn incoming_request_method(&mut self, this: types::IncomingRequest) -> Result<Method> {
        Ok(self
            .incoming_requests
            .get(this)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))?
            .method
            .clone())
    }

    async fn incoming_request_path_with_query(
        &mut self,
        this: types::IncomingRequest,
    ) -> Result<Option<String>> {
        Ok(self
            .incoming_requests
            .get(this)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))?
            .path_with_query
            .clone())
    }

    async fn incoming_request_scheme(
        &mut self,
        this: types::IncomingRequest,
    ) -> Result<Option<Scheme>> {
        Ok(self
            .incoming_requests
            .get(this)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))?
            .scheme
            .clone())
    }

    async fn incoming_request_authority(
        &mut self,
        this: types::IncomingRequest,
    ) -> Result<Option<String>> {
        Ok(self
            .incoming_requests
            .get(this)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))?
            .authority
            .clone())
    }

    async fn incoming_request_headers(
        &mut self,
        this: types::IncomingRequest,
    ) -> Result<types::Headers> {
        Ok(self
            .fields
            .push(
                self.incoming_requests
                    .get(this)
                    .ok_or_else(|| anyhow!("unknown handle: {this}"))?
                    .headers
                    .clone(),
            )
            .map_err(|()| anyhow!("table overflow"))?)
    }

    async fn incoming_request_consume(
        &mut self,
        this: types::IncomingRequest,
    ) -> Result<Result<types::IncomingStream, ()>> {
        let body = self
            .incoming_requests
            .get_mut(this)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))?
            .body
            .take();

        Ok(if let Some(body) = body {
            Ok(self
                .input_streams
                .push(InputStream {
                    chunk: None,
                    pollable: Pollable(Arc::new(AtomicBool::new(true))),
                    body,
                    end_of_stream: false,
                    trailers: None,
                })
                .map_err(|()| anyhow!("table overflow"))?)
        } else {
            Err(())
        })
    }

    async fn new_outgoing_request(
        &mut self,
        method: Method,
        path_with_query: Option<String>,
        scheme: Option<Scheme>,
        authority: Option<String>,
        headers: types::Headers,
    ) -> Result<types::OutgoingRequest> {
        let headers = self.fields_entries(headers).await?;

        let (sender, receiver) = mpsc::channel(1);

        Ok(self
            .outgoing_requests
            .push(OutgoingRequest {
                method,
                path_with_query,
                scheme,
                authority,
                headers,
                sender: Some(sender),
                receiver: Some(receiver),
            })
            .map_err(|()| anyhow!("table overflow"))?)
    }

    async fn outgoing_request_write(
        &mut self,
        this: types::OutgoingRequest,
    ) -> Result<Result<types::OutgoingStream, ()>> {
        let sender = self
            .outgoing_requests
            .get_mut(this)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))?
            .sender
            .take();

        Ok(if let Some(sender) = sender {
            Ok(self
                .output_streams
                .push(OutputStream {
                    pollable: Pollable(Arc::new(AtomicBool::new(true))),
                    sender: Sender::Reqwest(sender),
                })
                .map_err(|()| anyhow!("table overflow"))?)
        } else {
            Err(())
        })
    }

    async fn drop_response_outparam(&mut self, this: types::ResponseOutparam) -> Result<()> {
        self.response_outparams
            .remove(this)
            .map(drop)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))
    }

    async fn set_response_outparam(
        &mut self,
        this: types::ResponseOutparam,
        response: Result<types::OutgoingResponse, types::Error>,
    ) -> Result<Result<(), ()>> {
        let sender = self
            .response_outparams
            .get_mut(this)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))?
            .0
            .take();

        Ok(if let Some(sender) = sender {
            sender
                .send(match response {
                    Ok(response) => Ok(self
                        .outgoing_responses
                        .get_mut(response)
                        .ok_or_else(|| anyhow!("unknown handle: {response}"))?
                        .receiver()
                        .expect("response body should not yet have been taken")),

                    Err(error) => Err(error),
                })
                .expect("host should be listening for response");
            Ok(())
        } else {
            Err(())
        })
    }

    async fn drop_incoming_response(&mut self, this: types::IncomingResponse) -> Result<()> {
        self.incoming_responses
            .remove(this)
            .map(drop)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))
    }

    async fn drop_outgoing_response(&mut self, this: types::OutgoingResponse) -> Result<()> {
        self.outgoing_responses
            .remove(this)
            .map(drop)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))
    }

    async fn incoming_response_status(
        &mut self,
        this: types::IncomingResponse,
    ) -> Result<types::StatusCode> {
        Ok(self
            .incoming_responses
            .get(this)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))?
            .status)
    }

    async fn incoming_response_headers(
        &mut self,
        this: types::IncomingResponse,
    ) -> Result<types::Headers> {
        Ok(self
            .fields
            .push(
                self.incoming_responses
                    .get(this)
                    .ok_or_else(|| anyhow!("unknown handle: {this}"))?
                    .headers
                    .clone(),
            )
            .map_err(|()| anyhow!("table overflow"))?)
    }

    async fn incoming_response_consume(
        &mut self,
        this: types::IncomingResponse,
    ) -> Result<Result<types::IncomingStream, ()>> {
        let body = self
            .incoming_responses
            .get_mut(this)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))?
            .body
            .take();

        Ok(if let Some(body) = body {
            Ok(self
                .input_streams
                .push(InputStream {
                    chunk: None,
                    pollable: Pollable(Arc::new(AtomicBool::new(true))),
                    body,
                    end_of_stream: false,
                    trailers: None,
                })
                .map_err(|()| anyhow!("table overflow"))?)
        } else {
            Err(())
        })
    }

    async fn new_outgoing_response(
        &mut self,
        status: types::StatusCode,
        headers: types::Headers,
    ) -> Result<types::OutgoingResponse> {
        // TODO: What is supposed to happen if you create a new outgoing response with some headers and then edit
        // the headers?  Should the response reflect those changes?  Does the answer change depending on whether
        // they change before or after `outgoing_response_write` is called?  Here, we copy the headers, but I'm not
        // sure that's what is indended by the WIT interface.
        let (sender, body) = Body::channel();

        let headers = self.fields_entries(headers).await?;

        self.outgoing_responses
            .push(OutgoingResponse {
                status,
                headers,
                sender: Some(sender),
                body: Some(body),
            })
            .map_err(|()| anyhow!("table overflow"))
    }

    async fn outgoing_response_write(
        &mut self,
        this: types::OutgoingResponse,
    ) -> Result<Result<types::OutgoingStream, ()>> {
        let sender = self
            .outgoing_responses
            .get_mut(this)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))?
            .sender
            .take();

        Ok(if let Some(sender) = sender {
            Ok(self
                .output_streams
                .push(OutputStream {
                    pollable: Pollable(Arc::new(AtomicBool::new(true))),
                    sender: Sender::Hyper(Some(sender)),
                })
                .map_err(|()| anyhow!("table overflow"))?)
        } else {
            Err(())
        })
    }

    async fn drop_future_incoming_response(
        &mut self,
        this: types::FutureIncomingResponse,
    ) -> Result<()> {
        self.future_incoming_responses
            .remove(this)
            .map(drop)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))
    }

    async fn future_incoming_response_get(
        &mut self,
        this: types::FutureIncomingResponse,
    ) -> Result<Option<Result<types::IncomingResponse, types::Error>>> {
        let this = self
            .future_incoming_responses
            .get_mut(this)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))?;

        this.pollable.0.store(false, Ordering::SeqCst);

        Ok(
            match Pin::new(&mut this.response).poll(&mut Context::from_waker(
                &Arc::new(PollWaker {
                    pollable: this.pollable.clone(),
                    notify: self.notify.clone(),
                })
                .into_waker(),
            )) {
                Poll::Pending => None,
                Poll::Ready(response) => Some(match response {
                    Ok(response) => Ok(self
                        .incoming_responses
                        .push(response)
                        .map_err(|()| anyhow!("table overflow"))?),
                    Err(error) => Err(error),
                }),
            },
        )
    }

    async fn listen_to_future_incoming_response(
        &mut self,
        this: types::FutureIncomingResponse,
    ) -> Result<types::Pollable> {
        self.pollables
            .push(
                self.future_incoming_responses
                    .get_mut(this)
                    .ok_or_else(|| anyhow!("unknown handle: {this}"))?
                    .pollable
                    .clone(),
            )
            .map_err(|()| anyhow!("table overflow"))
    }
}

#[async_trait]
impl default_outgoing_http::Host for WasiCloud {
    async fn handle(
        &mut self,
        this: default_outgoing_http::OutgoingRequest,
        options: Option<default_outgoing_http::RequestOptions>,
    ) -> Result<default_outgoing_http::FutureIncomingResponse> {
        let request = self
            .outgoing_requests
            .get_mut(this)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))?;

        if options.is_some() {
            todo!("support outgoing request options")
        }

        let pollable = Pollable(Arc::new(AtomicBool::new(true)));

        let response = if let Some(receiver) = request.receiver.take() {
            self.client
                .request(
                    match &request.method {
                        Method::Get => reqwest::Method::GET,
                        Method::Post => reqwest::Method::POST,
                        Method::Put => reqwest::Method::PUT,
                        Method::Delete => reqwest::Method::DELETE,
                        Method::Patch => reqwest::Method::PATCH,
                        Method::Head => reqwest::Method::HEAD,
                        Method::Options => reqwest::Method::OPTIONS,
                        Method::Trace => reqwest::Method::TRACE,
                        Method::Connect => reqwest::Method::CONNECT,
                        Method::Other(s) => reqwest::Method::from_bytes(s.as_bytes())?,
                    },
                    format!(
                        "{}://{}{}",
                        request
                            .scheme
                            .as_ref()
                            .map(|scheme| match scheme {
                                Scheme::Http => "http",
                                Scheme::Https => "https",
                                Scheme::Other(s) => s,
                            })
                            .unwrap_or("http"),
                        request.authority.as_deref().unwrap_or(""),
                        request.path_with_query.as_deref().unwrap_or(""),
                    ),
                )
                .headers(
                    request
                        .headers
                        .iter()
                        .map(|(k, v)| Ok((HeaderName::from_str(k)?, HeaderValue::from_bytes(v)?)))
                        .collect::<Result<_>>()?,
                )
                .body(reqwest::Body::wrap_stream(receiver))
                .send()
                // TODO: Use a more specific error case where appropriate:
                .map_err(|e| types::Error::UnexpectedError(e.to_string()))
                .and_then(|response| {
                    future::ready(Ok(IncomingResponse {
                        status: response.status().as_u16(),
                        headers: Fields(Arc::new(Mutex::new(
                            response
                                .headers()
                                .iter()
                                .map(|(k, v)| (k.to_string(), v.as_bytes().to_vec()))
                                .collect(),
                        ))),
                        body: Some(Body::wrap_stream(response.bytes_stream().boxed())),
                    }))
                })
                .boxed()
        } else {
            // TODO: is this something we need to allow?
            future::ready(Err(types::Error::UnexpectedError(
                "unable to send the same request twice".into(),
            )))
            .boxed()
        };

        Ok(self
            .future_incoming_responses
            .push(FutureIncomingResponse { pollable, response })
            .map_err(|()| anyhow!("table overflow"))?)
    }
}
