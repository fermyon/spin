//! This test program takes argument(s) that determine which WASI feature to
//! exercise and returns an exit code of 0 for success, 1 for WASI interface
//! failure (which is sometimes expected in a test), and some other code on
//! invalid argument(s).

use std::time::Duration;

type Result = std::result::Result<(), Box<dyn std::error::Error>>;

fn main() -> Result {
    let mut args = std::env::args();
    let cmd = args.next().expect("cmd");
    match cmd.as_str() {
        "noop" => (),
        "echo" => {
            eprintln!("echo");
            std::io::copy(&mut std::io::stdin(), &mut std::io::stdout())?;
        }
        "alloc" => {
            let size: usize = args.next().expect("size").parse().expect("size");
            eprintln!("alloc {size}");
            let layout = std::alloc::Layout::from_size_align(size, 8).expect("layout");
            unsafe {
                let p = std::alloc::alloc(layout);
                if p.is_null() {
                    return Err("allocation failed".into());
                }
                // Force allocation to actually happen
                p.read_volatile();
            }
        }
        "read" => {
            let path = args.next().expect("path");
            eprintln!("read {path}");
            std::fs::read(path)?;
        }
        "write" => {
            let path = args.next().expect("path");
            eprintln!("write {path}");
            std::fs::write(path, "content")?;
        }
        "sleep" => {
            let duration =
                Duration::from_millis(args.next().expect("duration_ms").parse().expect("u64"));
            eprintln!("sleep {duration:?}");
            std::thread::sleep(duration);
        }
        "panic" => {
            eprintln!("panic");
            panic!("intentional panic");
        }
        cmd => panic!("unknown cmd {cmd}"),
    };
    Ok(())
}
