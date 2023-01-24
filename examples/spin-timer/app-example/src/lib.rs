wit_bindgen_rust::export!("../spin-timer.wit");

struct SpinTimer;

impl spin_timer::SpinTimer for SpinTimer {
    fn handle_timer_request() {
        let text = spin_sdk::config::get("message").unwrap();
        println!("{text}");
    }
}
