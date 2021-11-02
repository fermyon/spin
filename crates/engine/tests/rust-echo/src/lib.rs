witx_bindgen_rust::export!("../echo.witx");

struct Echo {}

impl echo::Echo for Echo {
    fn echo(msg: String) -> String {
        format!("Hello, {}", msg)
    }
}
