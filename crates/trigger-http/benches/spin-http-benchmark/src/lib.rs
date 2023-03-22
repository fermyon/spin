wit_bindgen_rust::export!("../../../../wit/ephemeral/spin-http.wit");

struct SpinHttp {}

impl spin_http::SpinHttp for SpinHttp {
    fn handle_http_request(req: spin_http::Request) -> spin_http::Response {
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
        spin_http::Response {
            status: 200,
            headers: None,
            body: None,
        }
    }
}
