fn main() {
    let spin_binary: std::path::PathBuf = std::env::args()
        .nth(1)
        .expect("expected first argument to be path to spin binary")
        .into();
    let config = conformance_tests::Config::new("canary");
    conformance_tests::run_tests(config, move |test| {
        conformance::run_test(test, &spin_binary)
    })
    .unwrap()
    .exit();
}
