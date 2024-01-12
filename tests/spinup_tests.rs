use reqwest::header::HeaderValue;
use std::path::PathBuf;

#[cfg(feature = "e2e-tests")]
mod testcases;

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
                testing_framework::Request::new(
                    reqwest::Method::GET,
                    &format!("/test?key={test_key}"),
                ),
                200,
                &[],
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
                testing_framework::Request::new(reqwest::Method::GET, "/test/hello"),
                200,
                &[],
                "I'm a teapot",
            )?;
            assert_spin_request(
                spin,
                testing_framework::Request::new(
                    reqwest::Method::GET,
                    "/test/hello/wildcards/should/be/handled",
                ),
                200,
                &[],
                "I'm a teapot",
            )?;
            assert_spin_request(
                spin,
                testing_framework::Request::new(reqwest::Method::GET, "/thishsouldfail"),
                404,
                &[],
                "",
            )?;
            assert_spin_request(
                spin,
                testing_framework::Request::new(reqwest::Method::GET, "/test/hello/test-placement"),
                200,
                &[],
                "text for test",
            )
        },
    )?;

    Ok(())
}

#[test]
// TODO: it seems that running this test on macOS CI is not possible because the docker services doesn't run.
// Investigate if there is a possible fix for this.
#[cfg(feature = "e2e-tests")]
/// Test that basic redis trigger support works
fn redis_smoke_test() -> anyhow::Result<()> {
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

    use anyhow::Context;
    use redis::Commands;
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

#[test]
/// Test dynamic environment variables
fn dynamic_env_test() -> anyhow::Result<()> {
    run_test(
        "dynamic-env-test",
        testing_framework::SpinMode::Http,
        vec!["--env".to_owned(), "foo=bar".to_owned()],
        testing_framework::ServicesConfig::none(),
        move |env| {
            let spin = env.runtime_mut();
            assert_spin_request(
                spin,
                testing_framework::Request::new(reqwest::Method::GET, "/env"),
                200,
                &[("env_some_key", "some_value"), ("ENV_foo", "bar")],
                "I'm a teapot",
            )?;
            Ok(())
        },
    )?;

    Ok(())
}

#[test]
/// Test that mounting works properly
fn assets_routing_test() -> anyhow::Result<()> {
    run_test(
        "assets-test",
        testing_framework::SpinMode::Http,
        [],
        testing_framework::ServicesConfig::none(),
        move |env| {
            let spin = env.runtime_mut();
            let mut assert_file = |name: &str, content: &str| {
                assert_spin_request(
                    spin,
                    testing_framework::Request::new(
                        reqwest::Method::GET,
                        &format!("/static/thisshouldbemounted/{name}"),
                    ),
                    200,
                    &[],
                    content,
                )
            };
            let mut assert_file_content_eq_name =
                |name: &str| assert_file(name, &format!("{name}\n"));

            assert_file_content_eq_name("1")?;
            assert_file_content_eq_name("2")?;
            assert_file_content_eq_name("3")?;
            assert_file("empty", "")?;
            assert_file("one-byte", "{")?;

            let mut assert_not_found = |path: &str| {
                assert_spin_request(
                    spin,
                    testing_framework::Request::new(
                        reqwest::Method::GET,
                        &format!("/static/{path}"),
                    ),
                    404,
                    &[],
                    "Not Found",
                )
            };

            assert_not_found("donotmount/a")?;
            assert_not_found("thisshouldbemounted/thisshouldbeexcluded/4")?;
            Ok(())
        },
    )?;

    Ok(())
}

#[test]
/// Test that mounting works properly
fn legacy_apps() -> anyhow::Result<()> {
    run_test(
        "legacy-apps-test",
        testing_framework::SpinMode::Http,
        [],
        testing_framework::ServicesConfig::none(),
        move |env| {
            let spin = env.runtime_mut();
            let mut test = |lang: &str, body: &str| {
                assert_spin_request(
                    spin,
                    testing_framework::Request::new(reqwest::Method::GET, &format!("/{lang}")),
                    200,
                    &[],
                    body,
                )
            };

            test("golang", "Hello Fermyon!\n")?;
            test("rust", "Hello, Fermyon")?;
            test("javascript", "Hello from JS-SDK")?;
            test("typescript", "Hello from TS-SDK")?;
            Ok(())
        },
    )?;

    Ok(())
}

#[test]
fn bad_build_test() -> anyhow::Result<()> {
    let mut env = bootstap_env(
        "error",
        [],
        testing_framework::ServicesConfig::none(),
        testing_framework::SpinMode::None,
    )?;
    let expected = r#"Error: Couldn't find trigger executor for local app "spin.toml"

Caused by:
      no triggers in app
"#;

    assert_eq!(env.runtime_mut().stderr(), expected);

    Ok(())
}

#[test]
fn outbound_http_works() -> anyhow::Result<()> {
    run_test(
        "outbound-http-to-same-app",
        testing_framework::SpinMode::Http,
        [],
        testing_framework::ServicesConfig::none(),
        move |env| {
            let spin = env.runtime_mut();
            assert_spin_request(
                spin,
                testing_framework::Request::new(reqwest::Method::GET, "/test/outbound-allowed"),
                200,
                &[],
                "Hello, Fermyon!\n",
            )?;

            assert_spin_request(
                spin,
                testing_framework::Request::new(reqwest::Method::GET, "/test/outbound-not-allowed"),
                500,
                &[],
                "",
            )?;

            Ok(())
        },
    )?;

    Ok(())
}

#[test]
fn http_python_template_smoke_test() -> anyhow::Result<()> {
    smoke_test_template(
        "http-py",
        Some("https://github.com/fermyon/spin-python-sdk"),
        &["py2wasm"],
        |_| Ok(()),
        "Hello from the Python SDK",
    )
}

#[test]
fn http_rust_template_smoke_test() -> anyhow::Result<()> {
    smoke_test_template("http-rust", None, &[], |_| Ok(()), "Hello, Fermyon")
}

#[test]
fn http_c_template_smoke_test() -> anyhow::Result<()> {
    smoke_test_template("http-c", None, &[], |_| Ok(()), "Hello from WAGI/1\n")
}

#[test]
fn http_go_template_smoke_test() -> anyhow::Result<()> {
    let prebuild = |env: &mut testing_framework::TestEnvironment<_>| {
        let mut tidy = std::process::Command::new("go");
        tidy.args(["mod", "tidy"]);
        env.run_in(&mut tidy)?;
        Ok(())
    };
    smoke_test_template("http-go", None, &[], prebuild, "Hello Fermyon!\n")
}

#[test]
fn http_js_template_smoke_test() -> anyhow::Result<()> {
    let prebuild = |env: &mut testing_framework::TestEnvironment<_>| {
        let mut tidy = std::process::Command::new("npm");
        tidy.args(["install"]);
        env.run_in(&mut tidy)?;
        Ok(())
    };
    smoke_test_template(
        "http-js",
        Some("https://github.com/fermyon/spin-js-sdk"),
        &["js2wasm"],
        prebuild,
        "Hello from JS-SDK",
    )
}

#[test]
fn http_ts_template_smoke_test() -> anyhow::Result<()> {
    let prebuild = |env: &mut testing_framework::TestEnvironment<_>| {
        let mut tidy = std::process::Command::new("npm");
        tidy.args(["install"]);
        env.run_in(&mut tidy)?;
        Ok(())
    };
    smoke_test_template(
        "http-ts",
        Some("https://github.com/fermyon/spin-js-sdk"),
        &["js2wasm"],
        prebuild,
        "Hello from TS-SDK",
    )
}

#[test]
#[cfg(target_arch = "x86_64")]
fn http_grain_template_smoke_test() -> anyhow::Result<()> {
    smoke_test_template("http-grain", None, &[], |_| Ok(()), "Hello, World\n")
}

#[test]
fn http_zig_template_smoke_test() -> anyhow::Result<()> {
    smoke_test_template("http-zig", None, &[], |_| Ok(()), "Hello World!\n")
}

#[test]
fn http_swift_template_smoke_test() -> anyhow::Result<()> {
    smoke_test_template("http-swift", None, &[], |_| Ok(()), "Hello from WAGI/1!\n")
}

#[test]
fn http_php_template_smoke_test() -> anyhow::Result<()> {
    smoke_test_template_with_route(
        "http-php",
        None,
        &[],
        |_| Ok(()),
        "/index.php",
        "Hello Fermyon Spin",
    )
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
    let mut env = bootstap_env(test_name, spin_up_args, services_config, mode)?;
    test(&mut env)?;
    Ok(())
}

/// Bootstrap a test environment
fn bootstap_env(
    test_name: impl Into<String>,
    spin_up_args: impl IntoIterator<Item = String>,
    services_config: testing_framework::ServicesConfig,
    mode: testing_framework::SpinMode,
) -> anyhow::Result<testing_framework::TestEnvironment<testing_framework::Spin>> {
    let test_name = test_name.into();
    let config = testing_framework::TestEnvironmentConfig::spin(
        spin_binary(),
        spin_up_args,
        move |env| preboot(&test_name, env),
        services_config,
        mode,
    );
    testing_framework::TestEnvironment::up(config)
}

/// Assert that a request to the spin server returns the expected status and body
fn assert_spin_request(
    spin: &mut testing_framework::Spin,
    request: testing_framework::Request<'_>,
    expected_status: u16,
    expected_headers: &[(&str, &str)],
    expected_body: &str,
) -> testing_framework::TestResult<anyhow::Error> {
    let uri = request.uri;
    let mut r = spin.make_http_request(request)?;
    let status = r.status();
    let headers = std::mem::take(r.headers_mut());
    let body = r.text().unwrap_or_else(|_| String::from("<non-utf8>"));
    if status != expected_status {
        return Err(testing_framework::TestError::Failure(anyhow::anyhow!(
            "Expected status {expected_status} for {uri} but got {status}\nBody:\n{body}",
        )));
    }
    let wrong_headers: std::collections::HashMap<_, _> = expected_headers
        .iter()
        .copied()
        .filter(|(ek, ev)| headers.get(*ek) != Some(&HeaderValue::from_str(ev).unwrap()))
        .collect();
    if !wrong_headers.is_empty() {
        return Err(testing_framework::TestError::Failure(anyhow::anyhow!(
            "Expected headers {headers:?}  to contain {wrong_headers:?}\nBody:\n{body}"
        )));
    }
    if body != expected_body {
        return Err(testing_framework::TestError::Failure(
            anyhow::anyhow!("expected body '{expected_body}', got '{body}'").into(),
        ));
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

/// Run a smoke test against a `spin new` template
fn smoke_test_template(
    template_name: &str,
    template_url: Option<&str>,
    plugins: &[&str],
    prebuild_hook: impl FnOnce(&mut testing_framework::TestEnvironment<()>) -> anyhow::Result<()>,
    expected_body: &str,
) -> anyhow::Result<()> {
    smoke_test_template_with_route(
        template_name,
        template_url,
        plugins,
        prebuild_hook,
        "/",
        expected_body,
    )
}

/// Run a smoke test against a given http route for a `spin new` template
fn smoke_test_template_with_route(
    template_name: &str,
    template_url: Option<&str>,
    plugins: &[&str],
    prebuild_hook: impl FnOnce(&mut testing_framework::TestEnvironment<()>) -> anyhow::Result<()>,
    route: &str,
    expected_body: &str,
) -> anyhow::Result<()> {
    let mut env: testing_framework::TestEnvironment<()> =
        testing_framework::TestEnvironment::boot(&testing_framework::ServicesConfig::none())?;
    if let Some(template_url) = template_url {
        let mut template_install = std::process::Command::new(spin_binary());
        template_install.args(["templates", "install", "--git", template_url, "--update"]);
        env.run_in(&mut template_install)?;
    }

    if !plugins.is_empty() {
        let mut plugin_update = std::process::Command::new(spin_binary());
        plugin_update.args(["plugin", "update"]);
        env.run_in(&mut plugin_update)?;
    }

    for plugin in plugins {
        let mut plugin_install = std::process::Command::new(spin_binary());
        plugin_install.args(["plugin", "install", plugin, "--yes"]);
        env.run_in(&mut plugin_install)?;
    }
    let mut new_app = std::process::Command::new(spin_binary());
    new_app.args([
        "new",
        "test",
        "-t",
        template_name,
        "-o",
        ".",
        "--accept-defaults",
    ]);
    env.run_in(&mut new_app)?;

    prebuild_hook(&mut env)?;
    let mut build = std::process::Command::new(spin_binary());
    build.args(["build"]);
    env.run_in(&mut build)?;
    let mut spin = testing_framework::Spin::start(
        &spin_binary(),
        &env,
        vec![] as Vec<&str>,
        testing_framework::SpinMode::Http,
    )?;

    assert_spin_request(
        &mut spin,
        testing_framework::Request::new(reqwest::Method::GET, route),
        200,
        &[],
        expected_body,
    )?;

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
}
