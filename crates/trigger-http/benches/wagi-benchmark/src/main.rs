fn main() {
    for arg in std::env::args() {
        // sleep=<ms> param simulates processing time
        if let Some(ms_str) = arg.strip_prefix("sleep=") {
            let ms = ms_str.parse().expect("invalid sleep");
            std::thread::sleep(std::time::Duration::from_millis(ms));
        }
    }

    println!("Content-Type: text/plain\n");
}
