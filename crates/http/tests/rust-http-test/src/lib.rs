use fermyon_http_v01::{Request, Response};

wai_bindgen_rust::export!("../../fermyon_http_v01.wai");

struct FermyonHttpV01 {}

impl fermyon_http_v01::FermyonHttpV01 for FermyonHttpV01 {
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
