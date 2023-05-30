use crate::{
    poll::{PollWaker, Pollable},
    wit::wasi::io::streams2::{self as streams, StreamStatus},
    WasiCloud,
};
use anyhow::{anyhow, Result};
use futures::{channel::mpsc, Future, Sink, SinkExt, Stream, TryStreamExt};
use http_crate::header::{HeaderMap, HeaderValue};
use hyper::{
    body::{self, Bytes, HttpBody},
    Body,
};
use spin_core::async_trait;
use std::{
    pin::Pin,
    sync::{atomic::Ordering, Arc},
    task::{Context, Poll},
};
use tokio::sync::Notify;

pub struct InputStream {
    pub chunk: Option<Bytes>,
    pub pollable: Pollable,
    pub body: Body,
    pub end_of_stream: bool,
    pub trailers: Option<HeaderMap<HeaderValue>>,
}

impl InputStream {
    async fn read(
        &mut self,
        len: u64,
        notify: Option<Arc<Notify>>,
    ) -> Result<(Vec<u8>, StreamStatus), streams::StreamError> {
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
                        .into(),
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

                break Ok((
                    Vec::new(),
                    if ended {
                        StreamStatus::Ended
                    } else {
                        StreamStatus::Open
                    },
                ));
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
                    Some((result, StreamStatus::Open))
                } else if let Some(notify) = notify.as_ref() {
                    self.pollable.0.store(false, Ordering::SeqCst);

                    match Pin::new(&mut self.body).poll_next(&mut Context::from_waker(
                        &Arc::new(PollWaker {
                            pollable: self.pollable.clone(),
                            notify: notify.clone(),
                        })
                        .into(),
                    )) {
                        Poll::Pending => Some((Vec::new(), StreamStatus::Open)),
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

pub enum Sender {
    Hyper(Option<body::Sender>),
    Reqwest(mpsc::Sender<Result<Bytes>>),
}

pub struct OutputStream {
    pub pollable: Pollable,
    pub sender: Sender,
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
            .into();

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

#[async_trait]
impl streams::Host for WasiCloud {
    async fn read(
        &mut self,
        this: streams::InputStream,
        len: u64,
    ) -> Result<Result<(Vec<u8>, StreamStatus), streams::StreamError>> {
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
    ) -> Result<Result<(Vec<u8>, StreamStatus), streams::StreamError>> {
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
    ) -> Result<Result<(u64, StreamStatus), streams::StreamError>> {
        _ = (this, len);
        todo!()
    }

    async fn blocking_skip(
        &mut self,
        this: streams::InputStream,
        len: u64,
    ) -> Result<Result<(u64, StreamStatus), streams::StreamError>> {
        _ = (this, len);
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
        _ = (this, len);
        todo!()
    }

    async fn blocking_write_zeroes(
        &mut self,
        this: streams::OutputStream,
        len: u64,
    ) -> Result<Result<u64, streams::StreamError>> {
        _ = (this, len);
        todo!()
    }

    async fn splice(
        &mut self,
        this: streams::OutputStream,
        src: streams::InputStream,
        len: u64,
    ) -> Result<Result<(u64, StreamStatus), streams::StreamError>> {
        _ = (this, src, len);
        todo!()
    }

    async fn blocking_splice(
        &mut self,
        this: streams::OutputStream,
        src: streams::InputStream,
        len: u64,
    ) -> Result<Result<(u64, StreamStatus), streams::StreamError>> {
        _ = (this, src, len);
        todo!()
    }

    async fn forward(
        &mut self,
        this: streams::OutputStream,
        src: streams::InputStream,
    ) -> Result<Result<u64, streams::StreamError>> {
        _ = (this, src);
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
