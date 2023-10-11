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
