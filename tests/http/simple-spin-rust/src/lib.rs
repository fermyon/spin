// Import the HTTP objects from the generated bindings.
use spin_http::{Request, Response};

// Generate Rust bindings for interface defined in spin-http.wit file
wit_bindgen_rust::export!("spin-http.wit");

struct SpinHttp {}
impl spin_http::SpinHttp for SpinHttp {
    // Implement the `handler` entrypoint for Spin HTTP components.
    fn handle_http_request(_req: Request) -> Response {
        Response {
            status: 200,
            headers: None,
            body: Some("I'm a teapot".as_bytes().to_vec()),
        }
    }
}
