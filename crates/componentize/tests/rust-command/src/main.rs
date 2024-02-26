use std::{env, io};

fn main() {
    for arg in env::args() {
        println!("{arg}");
    }

    io::copy(&mut io::stdin().lock(), &mut io::stdout().lock()).unwrap();
}
