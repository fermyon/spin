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
                    let request: ::spin_sdk::wasi_http::types::IncomingRequest = Into::into(request);
                    let response_out: ::spin_sdk::wasi_http::types::ResponseOutparam = Into::into(response_out);
                    let future = async move {
                        if let Err(e) = super::#func_name(request, response_out).await {
                            eprintln!("Handler returned an error: {e}");
                        }
                    };
                    futures::pin_mut!(future);
                    ::spin_sdk::wasi_http::executor::run(future);
                }
            }

            impl From<IncomingRequest> for  ::spin_sdk::wasi_http::types::IncomingRequest {
                fn from(req: IncomingRequest) -> Self {
                    let req = std::mem::ManuallyDrop::new(req);
                    unsafe { Self::from_handle(req.handle()) }
                }
            }

            impl From<ResponseOutparam> for  ::spin_sdk::wasi_http::types::ResponseOutparam {
                fn from(resp: ResponseOutparam) -> Self {
                    let resp = std::mem::ManuallyDrop::new(resp);
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
