wit_bindgen::generate!({
    world: "spin-timer",
    path: "..",
    exports: {
        world: MySpinTimer
    }
});

use fermyon::spin::config;

struct MySpinTimer;

impl Guest for MySpinTimer {
    fn handle_timer_request() {
        let text = config::get_config("message").unwrap();
        println!("{text}");
    }
}
