use spin_http_v01::{Request, Response};

wit_bindgen_rust::export!("../../spin_http_v01.wai");

struct SpinHttpV01 {}

impl spin_http_v01::SpinHttpV01 for SpinHttpV01 {
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
