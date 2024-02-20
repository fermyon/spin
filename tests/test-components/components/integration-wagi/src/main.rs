use std::{collections::HashMap, error::Error};

fn main() -> Result<(), Box<dyn Error>> {
    let mut body = "".to_string();
    match std::env::var("PATH_INFO")?.as_str() {
        // Echos request body to response body
        "/echo" => {
            body = std::io::read_to_string(std::io::stdin())?;
        }

        // Asserts that WAGI args match the JSON array in the request body
        "/assert-args" => {
            let expected: Vec<String> = stdin_json()?;
            let args = std::env::args().collect::<Vec<_>>();
            if args != expected {
                return Err(format!("expected args {expected:?} got {args:?}").into());
            }
        }

        // Asserts that env vars contains the JSON object entries in the request body
        "/assert-env" => {
            let expected: HashMap<String, String> = stdin_json()?;
            for (key, val) in expected {
                let got = std::env::var(&key)?;
                if got != val {
                    return Err(format!("expected env var {key}={val:?}, got {got:?}").into());
                }
            }
        }

        other => {
            return Err(format!("unknown test route {other:?}").into());
        }
    };

    print!("Content-Type: text/plain\n\n{body}");
    Ok(())
}

fn stdin_json<T: miniserde::Deserialize>() -> Result<T, Box<dyn Error>> {
    let body = std::io::read_to_string(std::io::stdin())?;
    Ok(miniserde::json::from_str(&body)?)
}
