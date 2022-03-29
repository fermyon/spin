// Generate Rust bindings for interfaces defined in WIT files
wit_bindgen_rust::export!("../../../wit/ephemeral/spin-http.wit");
wit_bindgen_rust::import!("../../../wit/ephemeral/spin-config.wit");

// Import the HTTP objects from the generated bindings.
use spin_http::{Request, Response};

struct SpinHttp {}
impl spin_http::SpinHttp for SpinHttp {
    // Implement the `handler` entrypoint for Spin HTTP components.
    fn handle_http_request(req: Request) -> Response {
        let path = req.uri;

        if path.contains("test-placement") {
            match std::fs::read_to_string("/test.txt") {
                Ok(text) => Response {
                    status: 200,
                    headers: None,
                    body: Some(text.as_bytes().to_vec()),
                },
                Err(e) => Response {
                    status: 500,
                    headers: None,
                    body: Some(format!("ERROR! {:?}", e).as_bytes().to_vec()),
                },
            }
        } else {
            let message =
                spin_config::get_config("message").expect("failed to get configured message");
            Response {
                status: 200,
                headers: None,
                body: Some(message.as_bytes().to_vec()),
            }
        }
    }
}
