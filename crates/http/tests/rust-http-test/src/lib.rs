use spin_http::{Request, Response};

wit_bindgen_rust::export!("../../../../wit/ephemeral/spin-http.wit");

struct SpinHttp {}

impl spin_http::SpinHttp for SpinHttp {
    fn handler(req: Request) -> Response {
        assert!(req.params.contains(&("abc".to_string(), "def".to_string())));

        assert!(req
            .headers
            .contains(&("x-custom-foo".to_string(), "bar".to_string())));
        assert!(req
            .headers
            .contains(&("x-custom-foo2".to_string(), "bar2".to_string())));

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
