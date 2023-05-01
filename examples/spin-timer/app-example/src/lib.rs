wit_bindgen::generate!({
    world: "spin-timer",
    path: "../spin-timer.wit"
});

struct MySpinTimer;

impl SpinTimer for MySpinTimer {
    fn handle_timer_request() -> ContinueTimer {
        let text = spin_sdk::config::get("message").unwrap();
        println!("{text}");

        // Return ContinueTimer::True if you want to continue the timer loop calling this component/function subsequently.
        ContinueTimer::True
    }
}

export_spin_timer!(MySpinTimer);
