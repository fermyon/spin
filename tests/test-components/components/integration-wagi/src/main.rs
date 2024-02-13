use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let body = match std::env::var("PATH_INFO")?.as_str() {
        "/hello" => "I'm a teapot".into(),
        "/echo" => std::io::read_to_string(std::io::stdin())?,
        "/args" => format!("{:?}", std::env::args().collect::<Vec<_>>()),
        "/env" => {
            let key = std::env::args().nth(1).unwrap_or_default();
            std::env::var(key)?
        }
        other => {
            println!("Content-Type: text/plain");
            println!("Status: 404\n\n");
            println!("Not Found (PATH_INFO={other:?})");
            return Ok(());
        }
    };
    print!("Content-Type: text/plain\n\n{body}");
    Ok(())
}
