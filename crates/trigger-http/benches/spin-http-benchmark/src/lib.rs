wit_bindgen::generate!({
    world: "http-trigger",
    path: "../../../../wit/preview2",
    exports: {
        "fermyon:spin/inbound-http": SpinHttp,
    }
});

use exports::fermyon::spin::inbound_http;

struct SpinHttp;

impl inbound_http::Guest for SpinHttp {
    fn handle_request(req: inbound_http::Request) -> inbound_http::Response {
        let params = req.uri.find('?').map(|i| &req.uri[i + 1..]).unwrap_or("");
        for (key, value) in url::form_urlencoded::parse(params.as_bytes()) {
            #[allow(clippy::single_match)]
            match &*key {
                // sleep=<ms> param simulates processing time
                "sleep" => {
                    let ms = value.parse().expect("invalid sleep");
                    std::thread::sleep(std::time::Duration::from_millis(ms));
                }
                // cpu=<ms> param simulates compute time
                "cpu" => {
                    let amt = value.parse().expect("invalid cpu");
                    for _ in 0..amt {
                        do_some_work();
                    }
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

// According to my computer, which is highly accurate, this is the best way to
// simulate precisely 1.5ms of work. That definitely won't change over time.
fn do_some_work() {
    const N: usize = 4096;
    const AMT: usize = 5_000;

    let mut a = [0u8; N];
    let mut b = [1u8; N];

    for _ in 0..AMT {
        a.copy_from_slice(&b);
        std::hint::black_box(&a);
        b.copy_from_slice(&a);
        std::hint::black_box(&b);
    }
}
