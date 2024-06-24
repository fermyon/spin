use anyhow::Context as _;
use testing_framework::runtimes::spin_cli::{SpinCli, SpinConfig};

fn main() {
    let spin_binary: std::path::PathBuf = std::env::args()
        .nth(1)
        .expect("expected first argument to be path to spin binary")
        .into();
    conformance_tests::run_tests(move |test| run_test(test, &spin_binary)).unwrap();
}

fn run_test(test: conformance_tests::Test, spin_binary: &std::path::Path) -> anyhow::Result<()> {
    let mut services = Vec::new();
    for precondition in test.config.preconditions {
        match precondition {
            conformance_tests::config::Precondition::HttpEcho => {
                services.push("http-echo".into());
            }
            conformance_tests::config::Precondition::TcpEcho => {
                services.push("tcp-echo".into());
            }
            conformance_tests::config::Precondition::KeyValueStore(_) => {}
        }
    }
    let env_config = SpinCli::config(
        SpinConfig {
            binary_path: spin_binary.to_owned(),
            spin_up_args: Vec::new(),
            app_type: testing_framework::runtimes::SpinAppType::Http,
        },
        test_environment::services::ServicesConfig::new(services)?,
        move |e| {
            let mut manifest =
                test_environment::manifest_template::EnvTemplate::from_file(&test.manifest)?;
            manifest.substitute(e, |_| None)?;
            e.write_file("spin.toml", manifest.contents())?;
            e.copy_into(&test.component, test.component.file_name().unwrap())?;
            Ok(())
        },
    );
    let mut env = test_environment::TestEnvironment::up(env_config, |_| Ok(()))?;
    for invocation in test.config.invocations {
        let conformance_tests::config::Invocation::Http(mut invocation) = invocation;
        invocation.request.substitute_from_env(&mut env)?;
        let spin = env.runtime_mut();
        let actual = invocation
            .request
            .send(|request| spin.make_http_request(request))?;

        conformance_tests::assertions::assert_response(&invocation.response, &actual)
            .with_context(|| {
                format!(
                    "Failed assertion.\nstdout: {}\nstderr: {}",
                    spin.stdout().to_owned(),
                    spin.stderr()
                )
            })?;
    }
    Ok(())
}
