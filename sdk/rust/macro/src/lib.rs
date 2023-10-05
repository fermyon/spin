use proc_macro::TokenStream;
use quote::quote;

const WIT_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/wit");

/// The entrypoint to a Spin HTTP component written in Rust.
#[proc_macro_attribute]
pub fn http_component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = syn::parse_macro_input!(item as syn::ItemFn);
    let func_name = &func.sig.ident;
    let preamble = preamble(Export::Http);

    quote!(
        #func
        mod __spin_http {
            #preamble
            impl self::exports::fermyon::spin::inbound_http::Guest for Spin {
                // Implement the `handler` entrypoint for Spin HTTP components.
                fn handle_request(req: self::exports::fermyon::spin::inbound_http::Request) -> self::exports::fermyon::spin::inbound_http::Response {
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
                            self::exports::fermyon::spin::inbound_http::Response {
                                status: 500,
                                headers: None,
                                body: Some(body.as_bytes().to_vec()),
                            }
                        },
                    }
                }
            }

            mod inbound_http_helpers {
                use super::fermyon::spin::http_types as spin_http_types;

                impl TryFrom<spin_http_types::Request> for http::Request<Option<bytes::Bytes>> {
                    type Error = anyhow::Error;

                    fn try_from(spin_req: spin_http_types::Request) -> Result<Self, Self::Error> {
                        let mut http_req = http::Request::builder()
                            .method(spin_req.method.clone())
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

                impl From<spin_http_types::Method> for http::Method {
                    fn from(spin_method: spin_http_types::Method) -> Self {
                        match spin_method {
                            spin_http_types::Method::Get => http::Method::GET,
                            spin_http_types::Method::Post => http::Method::POST,
                            spin_http_types::Method::Put => http::Method::PUT,
                            spin_http_types::Method::Delete => http::Method::DELETE,
                            spin_http_types::Method::Patch => http::Method::PATCH,
                            spin_http_types::Method::Head => http::Method::HEAD,
                            spin_http_types::Method::Options => http::Method::OPTIONS,
                        }
                    }
                }

                fn append_request_headers(
                    http_req: &mut http::request::Builder,
                    spin_req: &spin_http_types::Request,
                ) -> anyhow::Result<()> {
                    let headers = http_req.headers_mut().unwrap();
                    for (k, v) in &spin_req.headers {
                        headers.append(
                            <http::header::HeaderName as std::str::FromStr>::from_str(k)?,
                            http::header::HeaderValue::from_str(v)?,
                        );
                    }

                    Ok(())
                }

                impl TryFrom<spin_http_types::Response> for http::Response<Option<bytes::Bytes>> {
                    type Error = anyhow::Error;

                    fn try_from(spin_res: spin_http_types::Response) -> Result<Self, Self::Error> {
                        let mut http_res = http::Response::builder().status(spin_res.status);
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
                    http_res: &mut http::response::Builder,
                    spin_res: spin_http_types::Response,
                ) -> anyhow::Result<()> {
                    let headers = http_res.headers_mut().unwrap();
                    for (k, v) in spin_res.headers.unwrap() {
                        headers.append(
                            <http::header::HeaderName as ::std::str::FromStr>::from_str(&k)?,
                            http::header::HeaderValue::from_str(&v)?,
                        );
                    }

                    Ok(())
                }

                impl TryFrom<http::Response<Option<bytes::Bytes>>> for spin_http_types::Response {
                    type Error = anyhow::Error;

                    fn try_from(
                        http_res: http::Response<Option<bytes::Bytes>>,
                    ) -> Result<Self, Self::Error> {
                        let status = http_res.status().as_u16();
                        let headers = Some(outbound_headers(http_res.headers())?);
                        let body = http_res.body().as_ref().map(|b| b.to_vec());

                        Ok(spin_http_types::Response {
                            status,
                            headers,
                            body,
                        })
                    }
                }

                fn outbound_headers(hm: &http::HeaderMap) -> anyhow::Result<Vec<(String, String)>> {
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
    let preamble = preamble(Export::Redis);

    quote!(
        #func
        mod __spin_redis {
            #preamble
            impl self::exports::fermyon::spin::inbound_redis::Guest for Spin {
                fn handle_message(msg: self::exports::fermyon::spin::inbound_redis::Payload) -> Result<(), self::fermyon::spin::redis_types::Error> {
                    match super::#func_name(msg.try_into().expect("cannot convert from Spin Redis payload")) {
                        Ok(()) => Ok(()),
                        Err(e) => {
                            eprintln!("{}", e);
                            Err(self::fermyon::spin::redis_types::Error::Error)
                        },
                    }
                }
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
            world: "wasi-http-trigger",
            path: #WIT_PATH,
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

#[derive(Copy, Clone)]
enum Export {
    Http,
    Redis,
}

fn preamble(export: Export) -> proc_macro2::TokenStream {
    let export_decl = match export {
        Export::Http => quote!("fermyon:spin/inbound-http": Spin),
        Export::Redis => quote!("fermyon:spin/inbound-redis": Spin),
    };
    let world = match export {
        Export::Http => quote!("http-trigger"),
        Export::Redis => quote!("redis-trigger"),
    };
    quote! {
        #![allow(missing_docs)]
        ::spin_sdk::wit_bindgen::generate!({
            world: #world,
            path: #WIT_PATH,
            runtime_path: "::spin_sdk::wit_bindgen::rt",
            exports: {
                #export_decl
            }
        });
        struct Spin;
    }
}
