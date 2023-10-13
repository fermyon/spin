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
                    let req: ::spin_sdk::http::Request = ::std::convert::Into::into(req);
                    let req = match ::std::convert::TryInto::try_into(req) {
                        ::std::result::Result::Ok(r) => r,
                        ::std::result::Result::Err(e) => return ::std::convert::Into::into(::spin_sdk::http::IntoResponse::into_response(e)),
                    };
                    let resp = super::#func_name(req);
                    ::std::convert::Into::into(::spin_sdk::http::IntoResponse::into_response(resp))
                }
            }

            impl ::std::convert::From<self::fermyon::spin::http_types::Request> for ::spin_sdk::http::Request  {
                fn from(req: self::fermyon::spin::http_types::Request) -> Self {
                    Self {
                        method: ::std::convert::Into::into(req.method),
                        uri: req.uri,
                        params: req.params,
                        headers: req.headers,
                        body: req.body
                    }
                }
            }

            impl ::std::convert::From<self::fermyon::spin::http_types::Method> for ::spin_sdk::http::Method  {
                fn from(method: self::fermyon::spin::http_types::Method) -> Self {
                    match method {
                        self::fermyon::spin::http_types::Method::Get => Self::Get,
                        self::fermyon::spin::http_types::Method::Post => Self::Post,
                        self::fermyon::spin::http_types::Method::Put => Self::Put,
                        self::fermyon::spin::http_types::Method::Patch => Self::Patch,
                        self::fermyon::spin::http_types::Method::Delete => Self::Delete,
                        self::fermyon::spin::http_types::Method::Head => Self::Head,
                        self::fermyon::spin::http_types::Method::Options => Self::Options,
                    }
                }
            }

            impl ::std::convert::From<::spin_sdk::http::Response> for self::fermyon::spin::http_types::Response {
                fn from(resp: ::spin_sdk::http::Response) -> Self {
                    Self {
                        status: resp.status,
                        headers: resp.headers,
                        body: resp.body,
                    }
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
    let preamble = preamble(Export::WasiHttp);

    quote!(
        #func
        // We export wasi here since `wit-bindgen` currently has no way of using types
        // declared somewhere else as part of its generated code. If we want users to be able to
        // use `wasi-http` types, they have to be generated in this macro. This should be solved once
        // `with` is supported in wit-bindgen [ref: https://github.com/bytecodealliance/wit-bindgen/issues/694].
        use __spin_wasi_http::wasi;
        mod __spin_wasi_http {
            #preamble
            use exports::wasi::http::incoming_handler;
            use wasi::http::types::{IncomingRequest, ResponseOutparam};

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
    WasiHttp,
    Http,
    Redis,
}

fn preamble(export: Export) -> proc_macro2::TokenStream {
    let export_decl = match export {
        Export::WasiHttp => quote!("wasi:http/incoming-handler": Spin),
        Export::Http => quote!("fermyon:spin/inbound-http": Spin),
        Export::Redis => quote!("fermyon:spin/inbound-redis": Spin),
    };
    let world = match export {
        Export::WasiHttp => quote!("wasi-http-trigger"),
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
