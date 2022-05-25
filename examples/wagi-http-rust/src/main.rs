use miniserde::{json, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize)]
struct Output {
    args: Vec<String>,
    vars: BTreeMap<String, String>,
}

fn main() {
    eprintln!("WAGI-HTTP-RUST SEZ {:?}", std::env::var("X_FULL_URL"));
    println!(
        "Content-Type: application/json\n\n{}",
        json::to_string(&Output {
            args: std::env::args().collect(),
            vars: std::env::vars().collect(),
        })
    );
}
