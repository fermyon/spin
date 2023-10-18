use proc_macro::TokenStream;
use quote::quote;

const WIT_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/wit");

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
/// This form takes the form of a function with one `request` param and one `response` return value.
///
/// Requests are anything that implements `spin_sdk::http::conversions::TryFromIncomingRequest` which includes
/// `spin_sdk::http::Request`, `spin_sdk::http::IncomingRequest`, and even hyperium's popular `http` crate's `Request`
/// type.
///
/// Responses are anything that implements `spin_sdk::http::IntoResponse`. This includes `Result<impl IntoResponse, impl IntoResponse`,
/// `spin_sdk::http::Response`, and even the `http` crate's `Response` type.
///
/// For example:
/// ```ignore
/// use spin_sdk::http_component;
/// use spin_sdk::http::{Request, IntoResponse};
///
/// #[http_component]
/// async fn my_handler(request: Request) -> anyhow::Result<impl IntoResponse> {
///   // Your logic goes here
/// }
/// ```
///
/// ### Input/Output Params
///
/// Input/Output functions allow for streaming HTTP bodies. This form is by its very nature harder to use than
/// the request/response form above so it should only be favored when stream response bodies is desired.
///
/// The `request` param can be anything that implements `spin_sdk::http::TryFromIncomingRequest`. And
/// the `response_out` param must be a `spin_sdk::http::ResponseOutparam`. See the docs of `ResponseOutparam`
/// for how to use this type.
///
/// For example:
///
/// ```ignore
/// use spin_sdk::http_component;
/// use spin_sdk::http::{IncomingRequest, ResponseOutparam};
///
/// #[http_component]
/// async fn my_handler(request: IncomingRequest, response_out: ResponseOutparam) {
///   // Your logic goes here
/// }
/// ```
#[proc_macro_attribute]
pub fn http_component(_attr: TokenStream, item: TokenStream) -> TokenStream {
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
                    let request: ::spin_sdk::http::IncomingRequest = ::std::convert::Into::into(request);
                    let response_out: ::spin_sdk::http::ResponseOutparam = ::std::convert::Into::into(response_out);
                    ::spin_sdk::http::run(async move {
                        match ::spin_sdk::http::conversions::TryFromIncomingRequest::try_from_incoming_request(request).await {
                            ::std::result::Result::Ok(req) => #handler,
                            ::std::result::Result::Err(e) => handle_response(response_out, e).await,
                        }
                    });
                }
            }

            async fn handle_response<R: ::spin_sdk::http::IntoResponse>(response_out: ::spin_sdk::http::ResponseOutparam, resp: R) {
                let mut response = ::spin_sdk::http::IntoResponse::into_response(resp);
                let body = std::mem::take(&mut response.body);
                let response = ::std::convert::Into::into(response);
                if let Err(e) = ::spin_sdk::http::ResponseOutparam::set_with_body(response_out, response, body).await {
                    eprintln!("Could not set `ResponseOutparam`: {e}");
                }
            }

            impl From<self::wasi::http::types::IncomingRequest> for ::spin_sdk::http::IncomingRequest {
                fn from(req: self::wasi::http::types::IncomingRequest) -> Self {
                    let req = ::std::mem::ManuallyDrop::new(req);
                    unsafe { Self::from_handle(req.handle()) }
                }
            }

            impl From<::spin_sdk::http::OutgoingResponse> for self::wasi::http::types::OutgoingResponse {
                fn from(resp: ::spin_sdk::http::OutgoingResponse) -> Self {
                    unsafe { Self::from_handle(resp.into_handle()) }
                }
            }

            impl From<self::wasi::http::types::ResponseOutparam> for ::spin_sdk::http::ResponseOutparam {
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
    Redis,
}

fn preamble(export: Export) -> proc_macro2::TokenStream {
    let export_decl = match export {
        Export::WasiHttp => quote!("wasi:http/incoming-handler": Spin),
        Export::Redis => quote!("fermyon:spin/inbound-redis": Spin),
    };
    let world = match export {
        Export::WasiHttp => quote!("wasi-http-trigger"),
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
