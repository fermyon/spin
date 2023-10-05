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
