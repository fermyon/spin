wit_bindgen::generate!("spin-http" in "../../../../sdk/rust/macro/wit");

use inbound_http::{Request, Response};

struct SpinHttp;

export_spin_http!(SpinHttp);

#[export_name = "spin-sdk-version-1-2-pre0"]
extern "C" fn __spin_sdk_version() {}

impl inbound_http::InboundHttp for SpinHttp {
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
