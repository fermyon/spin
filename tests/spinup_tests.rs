use std::path::PathBuf;

#[cfg(feature = "e2e-tests")]
mod testcases;

fn spin_binary() -> PathBuf {
    env!("CARGO_BIN_EXE_spin").into()
}

#[test]
/// Test that the --key-value flag works as expected
fn key_value_cli_flag() -> anyhow::Result<()> {
    let test_key = uuid::Uuid::new_v4().to_string();
    let test_value = uuid::Uuid::new_v4().to_string();
    run_test(
        "key-value",
        ["--key-value".into(), format!("{test_key}={test_value}")],
        move |spin: &mut testing_framework::Spin| {
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

/// Run an e2e test
fn run_test(
    test_name: impl Into<String>,
    spin_up_args: impl IntoIterator<Item = String>,
    test: impl testing_framework::Test<Runtime = testing_framework::Spin, Failure = anyhow::Error>,
) -> testing_framework::TestResult<anyhow::Error> {
    let config = environment_config(test_name.into(), spin_up_args);
    testing_framework::TestEnvironment::up(config)?.test(test)?;
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

/// Get the configuration for a test environment
fn environment_config(
    test_name: String,
    spin_up_args: impl IntoIterator<Item = String>,
) -> testing_framework::TestEnvironmentConfig<testing_framework::Spin> {
    testing_framework::TestEnvironmentConfig::spin(
        spin_binary(),
        spin_up_args,
        move |env| preboot(&test_name, env),
        testing_framework::ServicesConfig::none(),
    )
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
    async fn head_rust_sdk_http() {
        testcases::head_rust_sdk_http(CONTROLLER).await
    }

    #[tokio::test]
    async fn head_rust_sdk_redis() {
        testcases::head_rust_sdk_redis(CONTROLLER).await
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
