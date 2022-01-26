use spin_http::{Request, Response};

wit_bindgen_rust::export!("../../../../wit/ephemeral/spin-http.wit");

struct SpinHttp {}

impl spin_http::SpinHttp for SpinHttp {
    fn handler(req: Request) -> Response {
        let body = Some(
            format!(
                "Hello, {}",
                std::str::from_utf8(&req.body.unwrap()).unwrap()
            )
            .as_bytes()
            .into(),
        );
        Response {
            status: 200,
            headers: None,
            body,
        }
    }
}
