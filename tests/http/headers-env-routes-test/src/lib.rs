// Import the HTTP objects from the generated bindings.
use spin_http::{Request, Response};

// Generate Rust bindings for interface defined in spin-http.wit file
wit_bindgen_rust::export!("spin-http.wit");

struct SpinHttp;
impl spin_http::SpinHttp for SpinHttp {
    // Implement the `handler` entrypoint for Spin HTTP components.
    // This handler does the following:
    // - returns all environment variables as headers with an ENV_ prefix
    // - returns all request headers as response headers.
    fn handle_http_request(req: Request) -> Response {
        let mut headers = Self::env_to_headers();
        Self::append_request_headers(&mut headers, &req.headers);
        let headers = Some(headers);
        Response {
            status: 200,
            headers,
            body: Some("I'm a teapot".as_bytes().to_vec()),
        }
    }
}

impl SpinHttp {
    fn env_to_headers() -> Vec<(String, String)> {
        let mut res = vec![];
        std::env::vars().for_each(|(k, v)| res.push((format!("ENV_{}", k), v)));

        res
    }

    fn append_request_headers(
        res_headers: &mut Vec<(String, String)>,
        req_headers: &[(String, String)],
    ) {
        for (k, v) in req_headers {
            res_headers.push((k.clone(), v.clone()));
        }
    }
}
