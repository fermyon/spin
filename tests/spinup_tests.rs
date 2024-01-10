use anyhow::Context;
use redis::Commands;
use std::path::PathBuf;

#[cfg(feature = "e2e-tests")]
mod testcases;

/// Helper macro to assert that a condition is true eventually
macro_rules! assert_eventually {
    ($e:expr) => {
        let mut i = 0;
        loop {
            if $e {
                break;
            } else if i > 20 {
                assert!($e);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
            i += 1;
        }
    };
}

#[test]
/// Test that the --key-value flag works as expected
fn key_value_cli_flag() -> anyhow::Result<()> {
    let test_key = uuid::Uuid::new_v4().to_string();
    let test_value = uuid::Uuid::new_v4().to_string();
    run_test(
        "key-value",
        testing_framework::SpinMode::Http,
        ["--key-value".into(), format!("{test_key}={test_value}")],
        testing_framework::ServicesConfig::none(),
        move |env| {
            let spin = env.runtime_mut();
            assert_spin_request(
                spin,
                reqwest::Method::GET,
                &format!("/test?key={test_key}"),
                200,
                &test_value,
            )
        },
    )?;
    Ok(())
}

#[test]
/// Test that basic http trigger support works
fn http_smoke_test() -> anyhow::Result<()> {
    run_test(
        "http-smoke-test",
        testing_framework::SpinMode::Http,
        [],
        testing_framework::ServicesConfig::none(),
        move |env| {
            let spin = env.runtime_mut();
            assert_spin_request(
                spin,
                reqwest::Method::GET,
                "/test/hello",
                200,
                "I'm a teapot",
            )?;
            assert_spin_request(
                spin,
                reqwest::Method::GET,
                "/test/hello/wildcards/should/be/handled",
                200,
                "I'm a teapot",
            )?;
            assert_spin_request(spin, reqwest::Method::GET, "/thishsouldfail", 404, "")?;
            assert_spin_request(
                spin,
                reqwest::Method::GET,
                "/test/hello/test-placement",
                200,
                "text for test",
            )
        },
    )?;

    Ok(())
}

#[test]
/// Test that basic redis trigger support works
fn redis_smoke_test() -> anyhow::Result<()> {
    run_test(
        "redis-smoke-test",
        testing_framework::SpinMode::Redis,
        [],
        testing_framework::ServicesConfig::new(vec!["redis".into()])?,
        move |env| {
            let redis_port = env
                .services_mut()
                .get_port(6379)?
                .context("no redis port was exposed by test services")?;

            let mut redis = redis::Client::open(format!("redis://localhost:{redis_port}"))
                .context("could not connect to redis in test")?;
            redis
                .publish("my-channel", "msg-from-test")
                .context("could not publish test message to redis")?;
            assert_eventually!({
                match env.read_file(".spin/logs/hello_stdout.txt") {
                    Ok(logs) => {
                        let logs = String::from_utf8_lossy(&logs);
                        logs.contains("Got message: 'msg-from-test'")
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
                    Err(e) => return Err(anyhow::anyhow!("could not read stdout file: {e}").into()),
                }
            });
            Ok(())
        },
    )?;

    Ok(())
}

/// Run an e2e test
fn run_test(
    test_name: impl Into<String>,
    mode: testing_framework::SpinMode,
    spin_up_args: impl IntoIterator<Item = String>,
    services_config: testing_framework::ServicesConfig,
    test: impl FnOnce(
            &mut testing_framework::TestEnvironment<testing_framework::Spin>,
        ) -> testing_framework::TestResult<anyhow::Error>
        + 'static,
) -> testing_framework::TestResult<anyhow::Error> {
    let test_name = test_name.into();
    let config = testing_framework::TestEnvironmentConfig::spin(
        spin_binary(),
        spin_up_args,
        move |env| preboot(&test_name, env),
        services_config,
        mode,
    );
    let mut env = testing_framework::TestEnvironment::up(config)?;
    test(&mut env)?;
    Ok(())
}

/// Assert that a request to the spin server returns the expected status and body
fn assert_spin_request(
    spin: &mut testing_framework::Spin,
    method: reqwest::Method,
    uri: &str,
    expected_status: u16,
    expected_body: &str,
) -> testing_framework::TestResult<anyhow::Error> {
    let r = spin.make_http_request(method, uri)?;
    let status = r.status();
    let body = r.text().unwrap_or_else(|_| String::from("<non-utf8>"));
    if status != expected_status {
        return Err(testing_framework::TestError::Failure(anyhow::anyhow!(
            "Expected status {expected_status} for {uri} but got {status}\nBody:\n{body}",
        )));
    }
    if body != expected_body {
        return Err(anyhow::anyhow!("expected {expected_body}, got {body}",).into());
    }
    Ok(())
}

/// Get the test environment ready to run a test
fn preboot(
    test: &str,
    env: &mut testing_framework::TestEnvironment<testing_framework::Spin>,
) -> anyhow::Result<()> {
    // Copy everything into the test environment
    env.copy_into(format!("tests/testcases/{test}"), "")?;

    // Copy the manifest with all templates substituted
    let manifest_path = PathBuf::from(format!("tests/testcases/{test}/spin.toml"));
    let mut template = testing_framework::ManifestTemplate::from_file(manifest_path)?;
    template.substitute(env)?;
    env.write_file("spin.toml", template.contents())?;
    Ok(())
}

/// Get the spin binary path
fn spin_binary() -> PathBuf {
    env!("CARGO_BIN_EXE_spin").into()
}

#[cfg(feature = "e2e-tests")]
mod spinup_tests {
    use super::testcases;
    use {e2e_testing::controller::Controller, e2e_testing::spin_controller::SpinUp};
    const CONTROLLER: &dyn Controller = &SpinUp {};

    #[tokio::test]
    async fn component_outbound_http_works() {
        testcases::component_outbound_http_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn http_go_works() {
        testcases::http_go_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn http_c_works() {
        testcases::http_c_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn http_rust_works() {
        testcases::http_rust_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn http_zig_works() {
        testcases::http_zig_works(CONTROLLER).await
    }

    #[tokio::test]
    #[cfg(target_arch = "x86_64")]
    async fn http_grain_works() {
        testcases::http_grain_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn http_ts_works() {
        testcases::http_ts_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn http_js_works() {
        testcases::http_js_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn http_python_works() {
        testcases::http_python_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn http_php_works() {
        testcases::http_php_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn http_swift_works() {
        testcases::http_swift_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn assets_routing_works() {
        testcases::assets_routing_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn llm_works() {
        testcases::llm_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn header_env_routes_works() {
        testcases::header_env_routes_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn header_dynamic_env_works() {
        testcases::header_dynamic_env_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn redis_go_works() {
        testcases::redis_go_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn redis_rust_works() {
        testcases::redis_rust_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn registry_works() {
        testcases::registry_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn longevity_apps_works() {
        testcases::longevity_apps_works(CONTROLLER).await
    }

    #[tokio::test]
    async fn error_messages() {
        testcases::error_messages(CONTROLLER).await
    }
}
