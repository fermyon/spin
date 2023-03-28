wit_bindgen::generate!({
    world: "spin-timer",
    path: "../spin-timer.wit"
});

struct MySpinTimer;

impl SpinTimer for MySpinTimer {
    fn handle_timer_request() {
        let text = spin_sdk::config::get("message").unwrap();
        println!("{text}");
    }
}

export_spin_timer!(MySpinTimer);
