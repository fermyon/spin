wit_bindgen_rust::export!("../echo.wit");

struct Echo;
impl echo::Echo for Echo {
    fn echo(msg: String) -> String {
        format!("ECHO: {}", msg)
    }
}
