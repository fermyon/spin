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
                fn handle_request(req: self::exports::fermyon::spin::inbound_http::Request) -> self::exports::fermyon::spin::inbound_http::Response {
                    let req: ::spin_sdk::http::Request = ::std::convert::Into::into(req);
                    let resp = match ::spin_sdk::http::TryFromRequest::try_from_request(req) {
                        ::std::result::Result::Ok(req) => ::spin_sdk::http::IntoResponse::into_response(super::#func_name(req)),
                        ::std::result::Result::Err(e) => ::spin_sdk::http::IntoResponse::into_response(e),
                    };
                    ::std::convert::Into::into(resp)
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
///
/// Functions annotated with this attribute can be of two forms:
/// * Request/Response
/// * Input/Output Params
///
/// When in doubt prefer the Request/Response variant unless streaming response bodies is something you need.
///
/// ### Request/Response
///
/// This form has the same shape as the `http_component` handlers. The only difference is that the underlying handling
/// happens through the `wasi-http` interface instead of the Spin specific `http` interface and thus requests are
/// anything that implements `spin_sdk::wasi_http::conversions::TryFromIncomingRequest` and responses are anything that
/// implements `spin_sdk::http::IntoResponse`.
///
/// For example:
/// ```rust
/// #[wasi_http_component]
/// async fn my_handler(request: IncomingRequest) -> anyhow::Result<impl IntoResponse> {
///   // Your logic goes here
/// }
/// ```
///
/// ### Input/Output Params
///
/// Input/Output functions allow for streaming HTTP bodies. They are expected generally to be in the form:
/// ```rust
/// #[wasi_http_component]
/// async fn my_handler(request: IncomingRequest, response_out: ResponseOutparam) {
///   // Your logic goes here
/// }
/// ```
///
/// The `request` param can be anything that implements `spin_sdk::wasi_http::conversions::TryFromIncomingRequest`.
/// This includes all types that implement `spin_sdk::http::TryIntoRequest` (which may be more convenient to use
/// when you don't need streaming request bodies).
#[proc_macro_attribute]
pub fn wasi_http_component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = syn::parse_macro_input!(item as syn::ItemFn);
    let func_name = &func.sig.ident;
    let preamble = preamble(Export::WasiHttp);
    let is_native_wasi_http_handler = func.sig.inputs.len() == 2;
    let await_postfix = func.sig.asyncness.map(|_| quote!(.await));
    let handler = if is_native_wasi_http_handler {
        quote! { super::#func_name(req, response_out)#await_postfix }
    } else {
        quote! { handle_response(response_out, super::#func_name(req)#await_postfix).await }
    };

    quote!(
        #func
        mod __spin_wasi_http {
            #preamble
            impl self::exports::wasi::http::incoming_handler::Guest for Spin {
                fn handle(request: wasi::http::types::IncomingRequest, response_out: self::wasi::http::types::ResponseOutparam) {
                    let request: ::spin_sdk::wasi_http::IncomingRequest = ::std::convert::Into::into(request);
                    let response_out: ::spin_sdk::wasi_http::ResponseOutparam = ::std::convert::Into::into(response_out);
                    ::spin_sdk::wasi_http::run(async move {
                        match ::spin_sdk::wasi_http::conversions::TryFromIncomingRequest::try_from_incoming_request(request).await {
                            ::std::result::Result::Ok(req) => #handler,
                            ::std::result::Result::Err(e) => handle_response(response_out, e).await,
                        }
                    });
                }
            }

            async fn handle_response<R: ::spin_sdk::http::IntoResponse>(response_out: ::spin_sdk::wasi_http::ResponseOutparam, resp: R) {
                let mut response = ::spin_sdk::http::IntoResponse::into_response(resp);
                let body = response.body.take().unwrap_or_default();
                let response = ::std::convert::Into::into(response);
                if let Err(e) = ::spin_sdk::wasi_http::ResponseOutparam::set_with_body(response_out, response, body).await {
                    eprintln!("Could not set `ResponseOutparam`: {e}");
                }
            }

            impl From<self::wasi::http::types::IncomingRequest> for ::spin_sdk::wasi_http::IncomingRequest {
                fn from(req: self::wasi::http::types::IncomingRequest) -> Self {
                    let req = ::std::mem::ManuallyDrop::new(req);
                    unsafe { Self::from_handle(req.handle()) }
                }
            }

            impl From<::spin_sdk::wasi_http::OutgoingResponse> for self::wasi::http::types::OutgoingResponse {
                fn from(resp: ::spin_sdk::wasi_http::OutgoingResponse) -> Self {
                    unsafe { Self::from_handle(resp.into_handle()) }
                }
            }

            impl From<self::wasi::http::types::ResponseOutparam> for ::spin_sdk::wasi_http::ResponseOutparam {
                fn from(resp: self::wasi::http::types::ResponseOutparam) -> Self {
                    let resp = ::std::mem::ManuallyDrop::new(resp);
                    unsafe { Self::from_handle(resp.handle()) }
                }
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
