wit_bindgen::generate!("http-trigger" in "../../../../wit");

use exports::fermyon::spin::inbound_http;

struct SpinHttp;
export_http_trigger!(SpinHttp);

impl inbound_http::InboundHttp for SpinHttp {
    fn handle_request(req: inbound_http::Request) -> inbound_http::Response {
        for param in req.params {
            #[allow(clippy::single_match)]
            match (param.0.as_str(), param.1) {
                // sleep=<ms> param simulates processing time
                ("sleep", ms_str) => {
                    let ms = ms_str.parse().expect("invalid sleep");
                    std::thread::sleep(std::time::Duration::from_millis(ms));
                }
                _ => (),
            }
        }
        inbound_http::Response {
            status: 200,
            headers: None,
            body: None,
        }
    }
}
