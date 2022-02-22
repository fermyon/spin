// Import the HTTP objects from the generated bindings.
use spin_http::{Request, Response};

// Generate Rust bindings for interface defined in spin-http.wit file
wit_bindgen_rust::export!("spin-http.wit");

wit_bindgen_rust::import!("spin-http-middleware-imports.wit");
wit_bindgen_rust::export!("spin-http-middleware-request.wit");
wit_bindgen_rust::export!("spin-http-middleware-response.wit");

struct SpinHttp {}
impl spin_http::SpinHttp for SpinHttp {
    // Implement the `handler` entrypoint for Spin HTTP components.
    fn handler(req: Request) -> Response {
        dbg!(req.method);
        Response {
            status: 200,
            headers: None,
            body: Some("I'm a teapot".as_bytes().to_vec()),
        }
    }
}

struct SpinHttpMiddlewareRequest {}
impl spin_http_middleware_request::SpinHttpMiddlewareRequest for SpinHttpMiddlewareRequest {
    fn intercept_request()->spin_http_middleware_request::InterceptRequestAction {
        let req =  spin_http_middleware_imports::request();
        req.set_method("POST");
        spin_http_middleware_request::InterceptRequestAction::Next
    }
}

struct SpinHttpMiddlewareResponse {}
impl spin_http_middleware_response::SpinHttpMiddlewareResponse for SpinHttpMiddlewareResponse {
    fn intercept_response() {
        let resp = spin_http_middleware_imports::response();
        resp.set_status(418);
    }
}