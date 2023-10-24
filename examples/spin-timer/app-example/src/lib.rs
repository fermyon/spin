wit_bindgen::generate!({
    world: "spin-timer",
    path: "..",
    exports: {
        world: MySpinTimer
    }
});

use fermyon::spin::variables;

struct MySpinTimer;

impl Guest for MySpinTimer {
    fn handle_timer_request() {
        let text = variables::get("message").unwrap();
        println!("{text}");
    }
}
