use proc_macro::TokenStream;
use quote::quote;

const HTTP_COMPONENT_WIT_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/wit");

/// The entrypoint to a Spin HTTP component written in Rust.
#[proc_macro_attribute]
pub fn http_component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = syn::parse_macro_input!(item as syn::ItemFn);
    let func_name = &func.sig.ident;

    quote!(
        #func

        mod __spin_http {
            ::spin_sdk::wit_bindgen::generate!({
                runtime_path: "::spin_sdk::wit_bindgen::rt",
                world: "reactor-http-export",
                path: #HTTP_COMPONENT_WIT_PATH,
                exports: {
                    "fermyon:spin/inbound-http": Spin,
                }
            });

            struct Spin;

            impl inbound_http::Guest for Spin {
                // Implement the `handler` entrypoint for Spin HTTP components.
                fn handle_request(req: inbound_http::Request) -> inbound_http::Response {
                    match super::#func_name(req.try_into().expect("cannot convert from Spin HTTP request")) {
                        Ok(resp) => resp.try_into().expect("cannot convert to Spin HTTP response"),
                        Err(error) => {
                            let body = error.to_string();
                            eprintln!("Handler returned an error: {}", body);
                            let mut source = error.source();
                            while let Some(s) = source {
                                eprintln!("  caused by: {}", s);
                                source = s.source();
                            }
                            inbound_http::Response {
                                status: 500,
                                headers: None,
                                body: Some(body.as_bytes().to_vec()),
                            }
                        },
                    }
                }
            }

            /// Inbound http trigger functionality
            // Hide the docs since this is only needed for the macro
            #[doc(hidden)]
            mod inbound_http {
                use super::exports::fermyon::spin::inbound_http;
                use super::fermyon::spin::http_types as spin_http_types;
                use ::spin_sdk::http_types;
                pub use inbound_http::*;

                impl TryFrom<inbound_http::Request> for http_types::Request<Option<bytes::Bytes>> {
                    type Error = anyhow::Error;

                    fn try_from(spin_req: inbound_http::Request) -> Result<Self, Self::Error> {
                        let mut http_req = http_types::Request::builder()
                            .method(spin_req.method)
                            .uri(&spin_req.uri);

                        append_request_headers(&mut http_req, &spin_req)?;

                        let body = match spin_req.body {
                            Some(b) => b.to_vec(),
                            None => Vec::new(),
                        };

                        let body = Some(bytes::Bytes::from(body));

                        Ok(http_req.body(body)?)
                    }
                }

                impl From<spin_http_types::Method> for http_types::Method {
                    fn from(spin_method: spin_http_types::Method) -> Self {
                        match spin_method {
                            spin_http_types::Method::Get => http_types::Method::GET,
                            spin_http_types::Method::Post => http_types::Method::POST,
                            spin_http_types::Method::Put => http_types::Method::PUT,
                            spin_http_types::Method::Delete => http_types::Method::DELETE,
                            spin_http_types::Method::Patch => http_types::Method::PATCH,
                            spin_http_types::Method::Head => http_types::Method::HEAD,
                            spin_http_types::Method::Options => http_types::Method::OPTIONS,
                        }
                    }
                }

                fn append_request_headers(
                    http_req: &mut http_types::request::Builder,
                    spin_req: &inbound_http::Request,
                ) -> anyhow::Result<()> {
                    let headers = http_req.headers_mut().unwrap();
                    for (k, v) in &spin_req.headers {
                        headers.append(
                            <http_types::header::HeaderName as std::str::FromStr>::from_str(k)?,
                            http_types::header::HeaderValue::from_str(v)?,
                        );
                    }

                    Ok(())
                }

                impl TryFrom<inbound_http::Response> for http_types::Response<Option<bytes::Bytes>> {
                    type Error = anyhow::Error;

                    fn try_from(spin_res: inbound_http::Response) -> Result<Self, Self::Error> {
                        let mut http_res = http_types::Response::builder().status(spin_res.status);
                        append_response_headers(&mut http_res, spin_res.clone())?;

                        let body = match spin_res.body {
                            Some(b) => b.to_vec(),
                            None => Vec::new(),
                        };
                        let body = Some(bytes::Bytes::from(body));

                        Ok(http_res.body(body)?)
                    }
                }

                fn append_response_headers(
                    http_res: &mut http_types::response::Builder,
                    spin_res: inbound_http::Response,
                ) -> anyhow::Result<()> {
                    let headers = http_res.headers_mut().unwrap();
                    for (k, v) in spin_res.headers.unwrap() {
                        headers.append(
                            <http_types::header::HeaderName as ::std::str::FromStr>::from_str(&k)?,
                            http_types::header::HeaderValue::from_str(&v)?,
                        );
                    }

                    Ok(())
                }

                impl TryFrom<http_types::Response<Option<bytes::Bytes>>> for inbound_http::Response {
                    type Error = anyhow::Error;

                    fn try_from(
                        http_res: http_types::Response<Option<bytes::Bytes>>,
                    ) -> Result<Self, Self::Error> {
                        let status = http_res.status().as_u16();
                        let headers = Some(outbound_headers(http_res.headers())?);
                        let body = http_res.body().as_ref().map(|b| b.to_vec());

                        Ok(inbound_http::Response {
                            status,
                            headers,
                            body,
                        })
                    }
                }

                fn outbound_headers(hm: &http_types::HeaderMap) -> anyhow::Result<Vec<(String, String)>> {
                    let mut res = Vec::new();

                    for (k, v) in hm {
                        res.push((
                            k.as_str().to_string(),
                            std::str::from_utf8(v.as_bytes())?.to_string(),
                        ));
                    }

                    Ok(res)
                }
            }
        }
    )
        .into()
}

/// Generates the entrypoint to a Spin Redis component written in Rust.
#[proc_macro_attribute]
pub fn redis_component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = syn::parse_macro_input!(item as syn::ItemFn);
    let func_name = &func.sig.ident;

    quote!(
        #func

        mod __spin_redis {
            ::spin_sdk::wit_bindgen::generate!({
                runtime_path: "::spin_sdk::wit_bindgen::rt",
                world: "reactor-redis-export",
                path: #HTTP_COMPONENT_WIT_PATH,
                exports: {
                    "fermyon:spin/inbound-redis": Spin,
                }
            });

            struct Spin;

            use fermyon::spin::redis_types;

            impl inbound_redis::Guest for Spin {
                fn handle_message(msg: inbound_redis::Payload) -> Result<(), redis_types::Error> {
                    match super::#func_name(msg.try_into().expect("cannot convert from Spin Redis payload")) {
                        Ok(()) => Ok(()),
                        Err(e) => {
                            eprintln!("{}", e);
                            Err(redis_types::Error::Error)
                        },
                    }
                }
            }

            /// Inbound redis trigger functionality
            // Hide the docs since this is only needed for the macro
            #[doc(hidden)]
            pub mod inbound_redis {
                pub use super::exports::fermyon::spin::inbound_redis::*;
            }
        }
    )
        .into()
}

/// The entrypoint to a WASI HTTP component written in Rust.
#[proc_macro_attribute]
pub fn wasi_http_component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = syn::parse_macro_input!(item as syn::ItemFn);
    let func_name = &func.sig.ident;

    quote!(
        ::spin_sdk::wit_bindgen::generate!({
            runtime_path: "::spin_sdk::wit_bindgen::rt",
            world: "reactor-wasi-http",
            path: #HTTP_COMPONENT_WIT_PATH,
            exports: {
                "wasi:http/incoming-handler": __spin_wasi_http::Spin,
            }
        });

        #func

        mod __spin_wasi_http {
            use super::{
                exports::wasi::http::incoming_handler,
                wasi::http::types::{IncomingRequest, ResponseOutparam}
            };

            pub struct Spin;

            impl incoming_handler::Guest for Spin {
                fn handle(request: IncomingRequest, response_out: ResponseOutparam) {
                    let future = async move {
                        if let Err(e) = super::#func_name(request, response_out).await {
                            eprintln!("Handler returned an error: {e}");
                        }
                    };
                    futures::pin_mut!(future);
                    super::executor::run(future);
                }
            }
        }

        mod executor {
            use {
                super::wasi::{
                    http::{
                        outgoing_handler,
                        types::{
                            self, IncomingBody, IncomingRequest, IncomingResponse, OutgoingBody,
                            OutgoingRequest, OutgoingResponse,
                        },
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
            pub fn outgoing_response_body(response: &OutgoingResponse) -> impl Sink<Vec<u8>, Error = Error> {
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
                                                let count =
                                                    usize::try_from(count).unwrap().min(chunk.len() - offset);

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
            pub fn incoming_response_body(response: IncomingResponse) -> impl Stream<Item = Result<Vec<u8>>> {
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
    )
        .into()
}
