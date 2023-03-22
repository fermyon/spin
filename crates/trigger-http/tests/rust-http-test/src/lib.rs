use spin_http::{Request, Response};

wit_bindgen_rust::export!("../../../../wit/ephemeral/spin-http.wit");

struct SpinHttp {}

impl spin_http::SpinHttp for SpinHttp {
    fn handle_http_request(req: Request) -> Response {
        assert!(req.params.is_empty());
        assert!(req.uri.contains("?abc=def"));

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
