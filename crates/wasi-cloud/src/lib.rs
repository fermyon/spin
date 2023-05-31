#![allow(unused)] // temporary, until `todo!()`s are filled in

use anyhow::{anyhow, Error, Result};
use cooked_waker::{IntoWaker, WakeRef};
use default_outgoing_http2 as default_outgoing_http;
use futures::{channel::oneshot, Stream, TryStreamExt};
use http_crate::header::{HeaderName, HeaderValue};
use hyper::{
    body::{self, Bytes, HttpBody},
    Body,
};
use poll2 as poll;
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
use tokio::sync::Notify;
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
pub struct Pollable(pub Arc<AtomicBool>);

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

pub struct OutgoingResponse {
    pub status: types::StatusCode,
    pub headers: FieldEntries,
    pub sender: Option<body::Sender>,
    pub body: Option<Body>,
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

pub struct InputStream {
    pub chunk: Option<Bytes>,
    pub pollable: Pollable,
    pub body: Body,
}

impl InputStream {
    async fn read(
        &mut self,
        len: u64,
        notify: Option<Arc<Notify>>,
    ) -> Result<(Vec<u8>, bool), streams::StreamError> {
        let len = usize::try_from(len).map_err(|_| streams::StreamError {})?;

        loop {
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
                    Poll::Ready(Some(chunk)) => {
                        let chunk = chunk.map_err(|_| streams::StreamError {})?;
                        self.chunk = if chunk.is_empty() { None } else { Some(chunk) };
                        None
                    }
                    Poll::Ready(None) => Some((Vec::new(), true)),
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
                Some((Vec::new(), true))
            };

            if let Some(result) = result {
                break Ok(result);
            }
        }
    }
}

pub struct OutputStream {
    pub pollable: Pollable,
    pub sender: body::Sender,
}

impl OutputStream {
    async fn write(
        &mut self,
        buf: Vec<u8>,
        notify: Option<Arc<Notify>>,
    ) -> Result<u64, streams::StreamError> {
        let len = u64::try_from(buf.len()).unwrap();

        if let Some(notify) = notify {
            self.pollable.0.store(false, Ordering::SeqCst);

            match self.sender.poll_ready(&mut Context::from_waker(
                &Arc::new(PollWaker {
                    pollable: self.pollable.clone(),
                    notify,
                })
                .into_waker(),
            )) {
                Poll::Pending => Ok(0),
                Poll::Ready(Err(_)) => Err(streams::StreamError {}),
                Poll::Ready(Ok(())) => match self.sender.try_send_data(buf.into()) {
                    Ok(()) => Ok(len),
                    Err(_) => Ok(0),
                },
            }
        } else {
            self.sender
                .send_data(buf.into())
                .await
                .map(|()| len)
                .map_err(|_| streams::StreamError {})
        }
    }
}

#[derive(Default)]
pub struct WasiCloud {
    pub incoming_requests: Table<IncomingRequest>,
    pub outgoing_responses: Table<OutgoingResponse>,
    pub fields: Table<Fields>,
    pub response_outparams: Table<ResponseOutparam>,
    pub pollables: Table<Pollable>,
    pub input_streams: Table<InputStream>,
    pub output_streams: Table<OutputStream>,
    notify: Arc<Notify>,
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
        // TODO: We should change the WIT file so this can return a `types::Error` on I/O error instead of trapping
        // TODO #2: Should there be a non-blocking version of this?
        self.input_streams
            .get_mut(this)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))?
            .body
            .trailers()
            .await?
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
        // See TODOs in `finish_incoming_stream`, which also apply here

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
            sender.send_trailers(trailers).await?;
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
        todo!()
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
        todo!()
    }

    async fn outgoing_request_write(
        &mut self,
        request: types::OutgoingRequest,
    ) -> Result<Result<types::OutgoingStream, ()>> {
        todo!()
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

    async fn drop_incoming_response(&mut self, response: types::IncomingResponse) -> Result<()> {
        todo!()
    }

    async fn drop_outgoing_response(&mut self, this: types::OutgoingResponse) -> Result<()> {
        self.outgoing_responses
            .remove(this)
            .map(drop)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))
    }

    async fn incoming_response_status(
        &mut self,
        response: types::IncomingResponse,
    ) -> Result<types::StatusCode> {
        todo!()
    }

    async fn incoming_response_headers(
        &mut self,
        response: types::IncomingResponse,
    ) -> Result<types::Headers> {
        todo!()
    }

    async fn incoming_response_consume(
        &mut self,
        response: types::IncomingResponse,
    ) -> Result<Result<types::IncomingStream, ()>> {
        todo!()
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
                    sender,
                })
                .map_err(|()| anyhow!("table overflow"))?)
        } else {
            Err(())
        })
    }

    async fn drop_future_incoming_response(
        &mut self,
        f: types::FutureIncomingResponse,
    ) -> Result<()> {
        todo!()
    }

    async fn future_incoming_response_get(
        &mut self,
        f: types::FutureIncomingResponse,
    ) -> Result<Option<Result<types::IncomingResponse, types::Error>>> {
        todo!()
    }

    async fn listen_to_future_incoming_response(
        &mut self,
        f: types::FutureIncomingResponse,
    ) -> Result<types::Pollable> {
        todo!()
    }
}

#[async_trait]
impl default_outgoing_http::Host for WasiCloud {
    async fn handle(
        &mut self,
        request: default_outgoing_http::OutgoingRequest,
        options: Option<default_outgoing_http::RequestOptions>,
    ) -> Result<default_outgoing_http::FutureIncomingResponse> {
        todo!()
    }
}
