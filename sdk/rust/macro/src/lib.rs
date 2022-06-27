use proc_macro::TokenStream;
use quote::quote;
use wit_bindgen_gen_core::{wit_parser::Interface, Direction, Files, Generator};
use wit_bindgen_gen_rust_wasm::RustWasm;

/// The entrypoint to a Spin HTTP component written in Rust.
#[proc_macro_attribute]
pub fn http_component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    const HTTP_COMPONENT_WIT_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/wit/spin-http.wit");

    let func = syn::parse_macro_input!(item as syn::ItemFn);
    let func_name = &func.sig.ident;

    let iface = Interface::parse_file(HTTP_COMPONENT_WIT_PATH)
        .expect("cannot parse Spin HTTP interface file");

    let mut files = Files::default();
    RustWasm::new().generate_one(&iface, Direction::Export, &mut files);
    let (_, contents) = files.iter().next().unwrap();
    let iface_tokens: TokenStream = std::str::from_utf8(contents)
        .expect("cannot parse UTF-8 from Spin HTTP interface file")
        .parse()
        .expect("cannot parse Spin HTTP interface file");
    let iface = syn::parse_macro_input!(iface_tokens as syn::ItemMod);

    quote!(
        #iface

        struct SpinHttp;
        impl spin_http::SpinHttp for SpinHttp {
            // Implement the `handler` entrypoint for Spin HTTP components.
            fn handle_http_request(req: spin_http::Request) -> spin_http::Response {
                #func

                match #func_name(req.try_into().expect("cannot convert from Spin HTTP request")) {
                    Ok(resp) => resp.try_into().expect("cannot convert to Spin HTTP response"),
                    Err(e) => {
                        let body = e.to_string();
                        eprintln!("Handler returned an error: {}", body);
                        spin_http::Response {
                            status: 500,
                            headers: None,
                            body: Some(body.as_bytes().to_vec()),
                        }
                    },
                }
            }
        }

        impl TryFrom<spin_http::Request> for http::Request<Option<bytes::Bytes>> {
            type Error = anyhow::Error;

            fn try_from(spin_req: spin_http::Request) -> Result<Self, Self::Error> {
                let mut http_req = http::Request::builder()
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

        impl From<spin_http::Method> for http::Method {
            fn from(spin_method: spin_http::Method) -> Self {
                match spin_method {
                    spin_http::Method::Get => http::Method::GET,
                    spin_http::Method::Post => http::Method::POST,
                    spin_http::Method::Put => http::Method::PUT,
                    spin_http::Method::Delete => http::Method::DELETE,
                    spin_http::Method::Patch => http::Method::PATCH,
                    spin_http::Method::Head => http::Method::HEAD,
                    spin_http::Method::Options => http::Method::OPTIONS,
                }
            }
        }

        fn append_request_headers(
            http_req: &mut http::request::Builder,
            spin_req: &spin_http::Request,
        ) -> anyhow::Result<()> {
            let headers = http_req.headers_mut().unwrap();
            for (k, v) in &spin_req.headers {
                headers.insert(
                    <http::header::HeaderName as std::str::FromStr>::from_str(k)?,
                    http::header::HeaderValue::from_str(v)?,
                );
            }

            Ok(())
        }

        impl TryFrom<spin_http::Response> for http::Response<Option<bytes::Bytes>> {
            type Error = anyhow::Error;

            fn try_from(spin_res: spin_http::Response) -> Result<Self, Self::Error> {
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
            spin_res: spin_http::Response,
        ) -> anyhow::Result<()> {
            let headers = http_res.headers_mut().unwrap();
            for (k, v) in spin_res.headers.unwrap() {
                headers.insert(
                    <http::header::HeaderName as std::str::FromStr>::from_str(&k)?,
                    http::header::HeaderValue::from_str(&v)?,
                );
            }

            Ok(())
        }

        impl TryFrom<spin_sdk::outbound_http::Response> for spin_http::Response {
            type Error = anyhow::Error;

            fn try_from(outbound_res: spin_sdk::outbound_http::Response) -> Result<Self, Self::Error> {
                let mut outbound_res = outbound_res;
                let status = outbound_res.status_code.as_u16();
                let body = Some(outbound_res.body_read_all()?);

                let headers = Some(outbound_headers(&outbound_res.headers_get_all()?)?);

                Ok(spin_http::Response {
                    status,
                    headers,
                    body,
                })
            }
        }

        impl TryFrom<http::Response<Option<bytes::Bytes>>> for spin_http::Response {
            type Error = anyhow::Error;

            fn try_from(http_res: http::Response<Option<bytes::Bytes>>) -> Result<Self, Self::Error> {
                let status = http_res.status().as_u16();
                let headers = Some(outbound_headers(http_res.headers())?);
                let body = http_res.body().as_ref().map(|b| b.to_vec());

                Ok(spin_http::Response {
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
    )
    .into()
}

/// Generates the entrypoint to a Spin Redis component written in Rust.
#[proc_macro_attribute]
pub fn redis_component(_attr: TokenStream, item: TokenStream) -> TokenStream {
    const REDIS_COMPONENT_WIT: &str =
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/wit/spin-redis.wit"));

    let func = syn::parse_macro_input!(item as syn::ItemFn);
    let func_name = &func.sig.ident;

    quote!(
        wit_bindgen_rust::export!({src["spin_redis"]: #REDIS_COMPONENT_WIT});

        struct SpinRedis;

        impl spin_redis::SpinRedis for SpinRedis {
            fn handle_redis_message(message: spin_redis::Payload) -> Result<(), spin_redis::Error> {
                #func

                match #func_name(message.try_into().expect("cannot convert from Spin Redis payload")) {
                    Ok(()) => Ok(()),
                    Err(e) => {
                        eprintln!("{}", e);
                        Err(spin_redis::Error::Error)
                    },
                }
            }
        }

    )
    .into()
}
