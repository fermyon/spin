wit_bindgen_rust::export!("../spin-timer.wit");

struct SpinTimer;
impl spin_timer::SpinTimer for SpinTimer {
    fn handle_timer_request(msg: String) -> String {
        format!("ECHO: {}", msg)
    }
}
