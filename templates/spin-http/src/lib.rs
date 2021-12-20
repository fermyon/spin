// Import the HTTP objects from the generated bindings.
use fermyon_http::{Request, Response};

// Generate Rust bindings for all interfaces in Cargo.toml.
wact::component!();

struct FermyonHttp {}
impl fermyon_http::FermyonHttp for FermyonHttp {
    // Implement the `handler` entrypoint for Spin HTTP components.
    fn handler(req: Request) -> Response {
        println!("Request: {:?}", req);
        Response {
            status: 418,
            headers: None,
            body: Some("I'm a teapot".as_bytes().to_vec()),
        }
    }
}
