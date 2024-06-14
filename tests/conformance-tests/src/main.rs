use testing_framework::runtimes::spin_cli::{SpinCli, SpinConfig};

fn main() {
    let spin_binary: std::path::PathBuf = std::env::args()
        .nth(1)
        .expect("expected first argument to be path to spin binary")
        .into();
    let tests_dir = conformance_tests::download_tests().unwrap();

    for test in conformance_tests::tests(&tests_dir).unwrap() {
        let env_config = SpinCli::config(
            SpinConfig {
                binary_path: spin_binary.clone(),
                spin_up_args: Vec::new(),
                app_type: testing_framework::runtimes::SpinAppType::Http,
            },
            test_environment::services::ServicesConfig::none(),
            move |e| {
                e.copy_into(&test.manifest, "spin.toml")?;
                e.copy_into(&test.component, test.component.file_name().unwrap())?;
                Ok(())
            },
        );
        let mut env = test_environment::TestEnvironment::up(env_config, |_| Ok(())).unwrap();
        let spin = env.runtime_mut();
        for invocation in test.config.invocations {
            let conformance_tests::config::Invocation::Http(invocation) = invocation;
            let actual = invocation
                .request
                .send(|request| spin.make_http_request(request))
                .unwrap();
            if let Err(e) =
                conformance_tests::assertions::assert_response(&invocation.response, &actual)
            {
                eprintln!("Test failed: {e}");
                eprintln!("stderr: {}", spin.stderr());
                std::process::exit(1);
            }
        }
    }
    println!("All tests passed!")
}
