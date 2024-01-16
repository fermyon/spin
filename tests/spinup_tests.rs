mod testcases;

mod spinup_tests {
    use super::testcases::{
        assert_spin_request, bootstap_env, bootstrap_smoke_test, http_smoke_test_template,
        http_smoke_test_template_with_route, redis_smoke_test_template, run_test, spin_binary,
    };
    use anyhow::Context;

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
                    testing_framework::Request::new(
                        reqwest::Method::GET,
                        "/test/hello/test-placement",
                    ),
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
    #[cfg(any(not(target_os = "macos"), feature = "e2e-tests"))]
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
                        Err(e) => {
                            return Err(anyhow::anyhow!("could not read stdout file: {e}").into())
                        }
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
                    testing_framework::Request::new(
                        reqwest::Method::GET,
                        "/test/outbound-not-allowed",
                    ),
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
    fn http_rust_template_smoke_test() -> anyhow::Result<()> {
        http_smoke_test_template("http-rust", None, &[], |_| Ok(()), "Hello, Fermyon")
    }

    #[test]
    fn test_vault_config_provider() -> anyhow::Result<()> {
        use std::collections::HashMap;
        const VAULT_ROOT_TOKEN: &str = "root";
        run_test(
            "vault-variables-test",
            testing_framework::SpinMode::Http,
            vec!["--runtime-config-file".into(), "runtime_config.toml".into()],
            testing_framework::ServicesConfig::new(vec!["vault".into()])?,
            |env| {
                let http_client = reqwest::blocking::Client::new();
                let body: HashMap<String, HashMap<String, String>> =
                    serde_json::from_value(serde_json::json!(
                        {
                            "data": {
                                "value": "test_password"
                            }

                        }
                    ))
                    .unwrap();
                let status = http_client
                    .post(format!(
                        "http://localhost:{}/v1/secret/data/password",
                        env.get_port(8200)?.context("vault port not found")?
                    ))
                    .header("X-Vault-Token", VAULT_ROOT_TOKEN)
                    .json(&body)
                    .send()
                    .context("failed to send request to Vault")?
                    .status();
                assert_eq!(status, 200);
                let spin = env.runtime_mut();
                let request = testing_framework::Request::new(reqwest::Method::GET, "/");
                assert_spin_request(spin, request, 200, &[], "Hello! Got password test_password")?;
                Ok(())
            },
        )?;

        Ok(())
    }

    #[test]
    #[cfg(feature = "e2e-tests")]
    fn http_python_template_smoke_test() -> anyhow::Result<()> {
        http_smoke_test_template(
            "http-py",
            Some("https://github.com/fermyon/spin-python-sdk"),
            &["py2wasm"],
            |_| Ok(()),
            "Hello from the Python SDK",
        )
    }

    #[test]
    #[cfg(feature = "e2e-tests")]
    fn http_c_template_smoke_test() -> anyhow::Result<()> {
        http_smoke_test_template("http-c", None, &[], |_| Ok(()), "Hello from WAGI/1\n")
    }

    #[test]
    #[cfg(feature = "e2e-tests")]
    fn http_go_template_smoke_test() -> anyhow::Result<()> {
        let prebuild = |env: &mut testing_framework::TestEnvironment<_>| {
            let mut tidy = std::process::Command::new("go");
            tidy.args(["mod", "tidy"]);
            env.run_in(&mut tidy)?;
            Ok(())
        };
        http_smoke_test_template("http-go", None, &[], prebuild, "Hello Fermyon!\n")
    }

    #[test]
    #[cfg(feature = "e2e-tests")]
    fn http_js_template_smoke_test() -> anyhow::Result<()> {
        let prebuild = |env: &mut testing_framework::TestEnvironment<_>| {
            let mut tidy = std::process::Command::new("npm");
            tidy.args(["install"]);
            env.run_in(&mut tidy)?;
            Ok(())
        };
        http_smoke_test_template(
            "http-js",
            Some("https://github.com/fermyon/spin-js-sdk"),
            &["js2wasm"],
            prebuild,
            "Hello from JS-SDK",
        )
    }

    #[test]
    #[cfg(feature = "e2e-tests")]
    fn http_ts_template_smoke_test() -> anyhow::Result<()> {
        let prebuild = |env: &mut testing_framework::TestEnvironment<_>| {
            let mut tidy = std::process::Command::new("npm");
            tidy.args(["install"]);
            env.run_in(&mut tidy)?;
            Ok(())
        };
        http_smoke_test_template(
            "http-ts",
            Some("https://github.com/fermyon/spin-js-sdk"),
            &["js2wasm"],
            prebuild,
            "Hello from TS-SDK",
        )
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    #[cfg(feature = "e2e-tests")]
    fn http_grain_template_smoke_test() -> anyhow::Result<()> {
        http_smoke_test_template("http-grain", None, &[], |_| Ok(()), "Hello, World\n")
    }

    #[test]
    #[cfg(feature = "e2e-tests")]
    fn http_zig_template_smoke_test() -> anyhow::Result<()> {
        http_smoke_test_template("http-zig", None, &[], |_| Ok(()), "Hello World!\n")
    }

    #[test]
    #[cfg(feature = "e2e-tests")]
    fn http_swift_template_smoke_test() -> anyhow::Result<()> {
        http_smoke_test_template("http-swift", None, &[], |_| Ok(()), "Hello from WAGI/1!\n")
    }

    #[test]
    fn http_php_template_smoke_test() -> anyhow::Result<()> {
        http_smoke_test_template_with_route(
            "http-php",
            None,
            &[],
            |_| Ok(()),
            "/index.php",
            "Hello Fermyon Spin",
        )
    }

    #[test]
    fn redis_go_template_smoke_test() -> anyhow::Result<()> {
        redis_smoke_test_template(
            "redis-go",
            None,
            &[],
            |port| {
                vec![
                    "--value".into(),
                    "redis-channel=redis-channel".into(),
                    "--value".into(),
                    format!("redis-address=redis://localhost:{port}"),
                ]
            },
            |env| {
                let mut tidy = std::process::Command::new("go");
                tidy.args(["mod", "tidy"]);
                env.run_in(&mut tidy)?;
                Ok(())
            },
        )
    }

    #[test]
    fn redis_rust_template_smoke_test() -> anyhow::Result<()> {
        redis_smoke_test_template(
            "redis-rust",
            None,
            &[],
            |port| {
                vec![
                    "--value".into(),
                    "redis-channel=redis-channel".into(),
                    "--value".into(),
                    format!("redis-address=redis://localhost:{port}"),
                ]
            },
            |_| Ok(()),
        )
    }

    #[test]
    fn registry_works() -> anyhow::Result<()> {
        let services = testing_framework::ServicesConfig::new(vec!["registry".into()])?;
        let spin_up_args = |env: &mut testing_framework::TestEnvironment<()>| {
            let registry_url = format!(
                "localhost:{}/spin-e2e-tests/registry-works/v1",
                env.get_port(5000)?
                    .context("no registry port was exposed by test services")?
            );
            let mut registry_push = std::process::Command::new(spin_binary());
            registry_push.args(["registry", "push", &registry_url, "--insecure"]);
            env.run_in(&mut registry_push)?;
            Ok(vec![
                "--from-registry".into(),
                registry_url,
                "--insecure".into(),
            ])
        };
        let mut env = bootstrap_smoke_test(
            &services,
            None,
            &[],
            "http-rust",
            |_| Ok(Vec::new()),
            |_| Ok(()),
            spin_up_args,
            testing_framework::SpinMode::Http,
        )?;
        assert_spin_request(
            env.runtime_mut(),
            testing_framework::Request::new(reqwest::Method::GET, "/"),
            200,
            &[],
            "Hello, Fermyon",
        )?;
        Ok(())
    }
}
