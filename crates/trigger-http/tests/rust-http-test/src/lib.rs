wit_bindgen::generate!({
    world: "http-trigger",
    path: "../../../../wit/preview2",
    exports: {
        "fermyon:spin/inbound-http": SpinHttp,
    }
});

use exports::fermyon::spin::inbound_http::{self, Request, Response};

struct SpinHttp;

impl inbound_http::Guest for SpinHttp {
    fn handle_request(req: Request) -> Response {
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
