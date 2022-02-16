// Import the HTTP objects from the generated bindings.
use spin_http::{Request, Response};

// Generate Rust bindings for interface defined in spin-http.wit file
wit_bindgen_rust::export!("spin-http.wit");
wit_bindgen_rust::export!("spin-http-interceptor.wit");

struct SpinHttp {}
impl spin_http::SpinHttp for SpinHttp {
    // Implement the `handler` entrypoint for Spin HTTP components.
    fn handler(req: Request) -> Response {
        Response {
            status: 200,
            headers: None,
            body: Some(format!("got {:?}", req.method).into()),
        }
    }
}

struct SpinHttpInterceptor {}
impl spin_http_interceptor::SpinHttpInterceptor for SpinHttpInterceptor {
    fn intercept_request(
        mut req: spin_http_interceptor::Request,
    ) -> spin_http_interceptor::InterceptRequestResult {
        req.method = spin_http_interceptor::Method::Post;
        spin_http_interceptor::InterceptRequestResult::Continue(req)
    }

    fn intercept_response(
        _resp: spin_http_interceptor::Response,
    ) -> spin_http_interceptor::Response {
        unimplemented!()
    }
}
