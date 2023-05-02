use proc_macro::TokenStream;
use quote::quote;

/// The entrypoint to a Spin HTTP component written in Rust.
#[proc_macro_attribute]
pub fn http_component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let func = syn::parse_macro_input!(item as syn::ItemFn);
    let func_name = &func.sig.ident;

    quote!(
        pub(crate) struct Spin;
        ::spin_sdk::export_spin!(Spin);

        impl ::spin_sdk::inbound_http::InboundHttp for Spin {
            // Implement the `handler` entrypoint for Spin HTTP components.
            fn handle_request(req: ::spin_sdk::inbound_http::Request) -> ::spin_sdk::inbound_http::Response {
                #func

                match #func_name(req.try_into().expect("cannot convert from Spin HTTP request")) {
                    Ok(resp) => resp.try_into().expect("cannot convert to Spin HTTP response"),
                    Err(error) => {
                        let body = error.to_string();
                        eprintln!("Handler returned an error: {}", body);
                        let mut source = error.source();
                        while let Some(s) = source {
                            eprintln!("  caused by: {}", s);
                            source = s.source();
                        }
                        ::spin_sdk::inbound_http::Response {
                            status: 500,
                            headers: None,
                            body: Some(body.as_bytes().to_vec()),
                        }
                    },
                }
            }
        }
        impl ::spin_sdk::inbound_redis::InboundRedis for Spin {
            fn handle_message(msg: ::spin_sdk::inbound_redis::Payload) -> Result<(), ::spin_sdk::inbound_redis::Error> {
                unimplemented!("No implementation for inbound-redis#handle-message");
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
        struct Spin;
        ::spin_sdk::export_spin!(Spin);

        impl ::spin_sdk::inbound_redis::InboundRedis for Spin {
            fn handle_message(message: ::spin_sdk::inbound_redis::Payload) -> Result<(), ::spin_sdk::inbound_redis::Error> {
                #func

                match #func_name(message.try_into().expect("cannot convert from Spin Redis payload")) {
                    Ok(()) => Ok(()),
                    Err(e) => {
                        eprintln!("{}", e);
                        Err(::spin_sdk::inbound_redis::Error::Error)
                    },
                }
            }
        }
        impl ::spin_sdk::inbound_http::InboundHttp for Spin {
            // Implement the `handler` entrypoint for Spin HTTP components.
            fn handle_request(req: ::spin_sdk::inbound_http::Request) -> ::spin_sdk::inbound_http::Response {
                unimplemented!("No implementation for inbound-http#handle-request");
            }
        }

    )
    .into()
}
