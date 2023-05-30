use crate::{
    poll::{PollWaker, Pollable},
    streams::{InputStream, OutputStream, Sender},
    wit::wasi::http::{
        outgoing_handler,
        types2::{self as types, Method, Scheme},
    },
    WasiCloud,
};
use anyhow::{anyhow, bail, Result};
use futures::{
    channel::{mpsc, oneshot},
    future::{self, BoxFuture},
    Future, FutureExt, StreamExt, TryFutureExt,
};
use http_crate::header::{HeaderName, HeaderValue};
use hyper::{
    body::{self, Bytes},
    Body,
};
use spin_core::async_trait;
use std::{
    pin::Pin,
    str::FromStr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    task::{Context, Poll},
};

pub struct IncomingRequest {
    pub method: Method,
    pub path_with_query: Option<String>,
    pub scheme: Option<Scheme>,
    pub authority: Option<String>,
    pub headers: Fields,
    pub body: Option<Body>,
}

type FieldEntries = Vec<(String, Vec<u8>)>;

#[derive(Clone, Debug)]
pub struct Fields(pub Arc<Mutex<FieldEntries>>);

pub struct ResponseOutparam(
    pub Option<oneshot::Sender<Result<OutgoingResponseReceiver, types::Error>>>,
);

pub struct OutgoingResponse {
    status: types::StatusCode,
    headers: Fields,
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
    pub headers: Fields,
    pub body: Body,
}

pub struct OutgoingRequest {
    method: Method,
    path_with_query: Option<String>,
    scheme: Option<Scheme>,
    authority: Option<String>,
    headers: FieldEntries,
    sender: Option<mpsc::Sender<Result<Bytes>>>,
    receiver: Option<mpsc::Receiver<Result<Bytes>>>,
}

pub struct IncomingResponse {
    status: types::StatusCode,
    headers: Fields,
    body: Option<Body>,
}

pub struct FutureIncomingResponse {
    pollable: Pollable,
    response: BoxFuture<'static, Result<IncomingResponse, types::Error>>,
}

pub struct FutureWriteTrailersResult {
    pollable: Pollable,
    result: BoxFuture<'static, Result<(), types::Error>>,
}

pub struct FutureTrailers {
    pollable: Pollable,
    fields: BoxFuture<'static, Result<Fields, types::Error>>,
}

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
        self.fields
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

    async fn finish_outgoing_stream(&mut self, this: types::OutgoingStream) -> Result<()> {
        let sender = &mut self
            .output_streams
            .get_mut(this)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))?
            .sender;

        match sender {
            Sender::Hyper(sender) => {
                if sender.take().is_none() {
                    bail!("stream already finished");
                }
            }
            Sender::Reqwest(_) => (),
        }

        Ok(())
    }

    async fn finish_outgoing_stream_with_trailers(
        &mut self,
        this: types::OutgoingStream,
        trailers: types::Trailers,
    ) -> Result<types::FutureWriteTrailersResult> {
        // TODO: return a future that resolves to an error instead of trapping when the trailers can't be parsed.
        let trailers = self
            .fields
            .get(trailers)
            .ok_or_else(|| anyhow!("unknown handle: {trailers}"))?
            .0
            .lock()
            .unwrap()
            .iter()
            .map(|(k, v)| Ok((HeaderName::from_str(k)?, HeaderValue::from_bytes(v)?)))
            .collect::<Result<_>>()?;

        let sender = &mut self
            .output_streams
            .get_mut(this)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))?
            .sender;

        match sender {
            Sender::Hyper(sender) => {
                if let Some(mut sender) = sender.take() {
                    let pollable = Pollable(Arc::new(AtomicBool::new(true)));
                    Ok(self
                        .future_write_trailers_results
                        .push(FutureWriteTrailersResult {
                            pollable,
                            // TODO: inspect error and use a more precise variant than `UnexpectedError`
                            result: async move { sender.send_trailers(trailers).await }
                                .map_err(|e| types::Error::UnexpectedError(e.to_string()))
                                .boxed(),
                        })
                        .map_err(|()| anyhow!("table overflow"))?)
                } else {
                    bail!("stream already finished");
                }
            }
            // TODO: will probably need to contribute trailer support upstream to `reqwest` or else use `hyper`
            // directly:
            Sender::Reqwest(_) => bail!("trailers not yet supported for outgoing requests"),
        }
    }

    async fn drop_future_write_trailers_result(
        &mut self,
        this: types::FutureWriteTrailersResult,
    ) -> Result<()> {
        self.future_write_trailers_results
            .remove(this)
            .map(drop)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))
    }

    async fn future_write_trailers_result_get(
        &mut self,
        this: types::FutureWriteTrailersResult,
    ) -> Result<Option<Result<(), types::Error>>> {
        let this = self
            .future_write_trailers_results
            .get_mut(this)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))?;

        this.pollable.0.store(false, Ordering::SeqCst);

        Ok(
            match Pin::new(&mut this.result).poll(&mut Context::from_waker(
                &Arc::new(PollWaker {
                    pollable: this.pollable.clone(),
                    notify: self.notify.clone(),
                })
                .into(),
            )) {
                Poll::Pending => None,
                Poll::Ready(result) => Some(result),
            },
        )
    }

    async fn listen_to_future_write_trailers_result(
        &mut self,
        this: types::FutureWriteTrailersResult,
    ) -> Result<types::Pollable> {
        self.pollables
            .push(
                self.future_write_trailers_results
                    .get_mut(this)
                    .ok_or_else(|| anyhow!("unknown handle: {this}"))?
                    .pollable
                    .clone(),
            )
            .map_err(|()| anyhow!("table overflow"))
    }

    async fn drop_future_trailers(&mut self, this: types::FutureTrailers) -> Result<()> {
        self.future_trailers
            .remove(this)
            .map(drop)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))
    }

    async fn future_trailers_get(
        &mut self,
        this: types::FutureTrailers,
    ) -> Result<Option<Result<types::Trailers, types::Error>>> {
        let this = self
            .future_trailers
            .get_mut(this)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))?;

        this.pollable.0.store(false, Ordering::SeqCst);

        Ok(
            match Pin::new(&mut this.fields).poll(&mut Context::from_waker(
                &Arc::new(PollWaker {
                    pollable: this.pollable.clone(),
                    notify: self.notify.clone(),
                })
                .into(),
            )) {
                Poll::Pending => None,
                Poll::Ready(fields) => Some(match fields {
                    Ok(fields) => Ok(self
                        .fields
                        .push(fields)
                        .map_err(|()| anyhow!("table overflow"))?),
                    Err(error) => Err(error),
                }),
            },
        )
    }

    async fn listen_to_future_trailers(
        &mut self,
        this: types::FutureTrailers,
    ) -> Result<types::Pollable> {
        self.pollables
            .push(
                self.future_trailers
                    .get_mut(this)
                    .ok_or_else(|| anyhow!("unknown handle: {this}"))?
                    .pollable
                    .clone(),
            )
            .map_err(|()| anyhow!("table overflow"))
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
    ) -> Result<Result<types::OutgoingRequest, types::Error>> {
        let headers = self.fields_entries(headers).await?;

        let (sender, receiver) = mpsc::channel(1);

        Ok(Ok(self
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
            .map_err(|()| anyhow!("table overflow"))?))
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
    ) -> Result<Result<types::OutgoingResponse, types::Error>> {
        let (sender, body) = Body::channel();

        let headers = self
            .fields
            .get(headers)
            .ok_or_else(|| anyhow!("unknown handle: {headers}"))?
            .clone();

        Ok(Ok(self
            .outgoing_responses
            .push(OutgoingResponse {
                status,
                headers,
                sender: Some(sender),
                body: Some(body),
            })
            .map_err(|()| anyhow!("table overflow"))?))
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
                .into(),
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
impl outgoing_handler::Host for WasiCloud {
    async fn handle(
        &mut self,
        this: outgoing_handler::OutgoingRequest,
        options: Option<outgoing_handler::RequestOptions>,
    ) -> Result<outgoing_handler::FutureIncomingResponse> {
        let request = self
            .outgoing_requests
            .get_mut(this)
            .ok_or_else(|| anyhow!("unknown handle: {this}"))?;

        if options.is_some() {
            todo!("support outgoing request options")
        }

        let pollable = Pollable(Arc::new(AtomicBool::new(true)));

        let response = if let Some(receiver) = request.receiver.take() {
            self.http_client
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
