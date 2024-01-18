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
                        Err(e)
                            if e.downcast_ref()
                                .map(|e: &std::io::Error| e.kind() == std::io::ErrorKind::NotFound)
                                .unwrap_or_default() =>
                        {
                            false
                        }
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
    fn test_simple_rust_local() -> anyhow::Result<()> {
        run_test(
            "simple-test",
            testing_framework::SpinMode::Http,
            [],
            testing_framework::ServicesConfig::none(),
            |env| {
                let spin = env.runtime_mut();
                let mut ensure_success = |uri, expected_status, expected_body| {
                    let request = testing_framework::Request::new(reqwest::Method::GET, uri);
                    assert_spin_request(spin, request, expected_status, &[], expected_body)
                };
                ensure_success("/test/hello", 200, "I'm a teapot")?;
                ensure_success(
                    "/test/hello/wildcards/should/be/handled",
                    200,
                    "I'm a teapot",
                )?;
                ensure_success("/thisshouldfail", 404, "")?;
                ensure_success("/test/hello/test-placement", 200, "text for test")?;
                Ok(())
            },
        )?;

        Ok(())
    }

    #[test]
    fn test_duplicate_rust_local() -> anyhow::Result<()> {
        run_test(
            "simple-double-test",
            testing_framework::SpinMode::Http,
            [],
            testing_framework::ServicesConfig::none(),
            |env| {
                let spin = env.runtime_mut();
                let mut ensure_success = |uri, expected_status, expected_body| {
                    let request = testing_framework::Request::new(reqwest::Method::GET, uri);
                    assert_spin_request(spin, request, expected_status, &[], expected_body)
                };
                ensure_success("/route1", 200, "I'm a teapot")?;
                ensure_success("/route2", 200, "I'm a teapot")?;
                ensure_success("/thisshouldfail", 404, "")?;
                Ok(())
            },
        )?;

        Ok(())
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

    #[test]
    fn test_wasi_http_rc_11_10() -> anyhow::Result<()> {
        test_wasi_http_rc("wasi-http-0.2.0-rc-2023-11-10")
    }

    #[test]
    fn test_wasi_http_rc_12_05() -> anyhow::Result<()> {
        test_wasi_http_rc("wasi-http-0.2.0-rc-2023-12-05")
    }

    fn test_wasi_http_rc(test_name: &str) -> anyhow::Result<()> {
        let body = "So rested he by the Tumtum tree";

        run_test(
            test_name,
            testing_framework::SpinMode::Http,
            [],
            testing_framework::ServicesConfig::new(vec!["http-echo".into()])?,
            |env| {
                let port = env
                    .get_port(80)?
                    .context("no http-echo port was exposed by test services")?;
                assert_spin_request(
                    env.runtime_mut(),
                    testing_framework::Request::full(
                        reqwest::Method::GET,
                        "/",
                        &[("url", &format!("http://127.0.0.1:{port}/",))],
                        Some(body.into()),
                    ),
                    200,
                    &[],
                    "Hello, world!",
                )?;
                Ok(())
            },
        )?;

        Ok(())
    }

    #[test]
    fn spin_up_gives_help_on_new_app() -> anyhow::Result<()> {
        let env = testing_framework::TestEnvironment::<()>::boot(
            &testing_framework::ServicesConfig::none(),
        )?;

        // We still don't see full help if there are no components.
        let toml_text = r#"spin_version = "1"
name = "unbuilt"
trigger = { type = "http" }
version = "0.1.0"
[[component]]
id = "unbuilt"
source = "fake.wasm"
[component.trigger]
route = "/..."
"#;
        env.write_file("spin.toml", toml_text)?;
        env.write_file("fake.wasm", [])?;

        testing_framework::Spin::start(
            &spin_binary(),
            &env,
            Vec::<String>::new(),
            testing_framework::SpinMode::None,
        )?;

        let mut up = std::process::Command::new(spin_binary());
        up.args(["up", "--help"]);
        let output = env.run_in(&mut up)?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("--quiet"));
        assert!(stdout.contains("--listen"));

        Ok(())
    }

    // TODO: Test on Windows
    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_spin_plugin_install_command() -> anyhow::Result<()> {
        let env = testing_framework::TestEnvironment::<()>::boot(
            &testing_framework::ServicesConfig::none(),
        )?;

        let path_to_test_dir = std::env::current_dir()?;
        let file_url = format!(
            "file:{}/tests/testcases/plugin/example.tar.gz",
            path_to_test_dir.to_str().unwrap()
        );
        let mut plugin_manifest_json = serde_json::json!(
        {
            "name": "example",
            "description": "A description of the plugin.",
            "homepage": "www.example.com",
            "version": "0.2.0",
            "spinCompatibility": ">=0.5",
            "license": "MIT",
            "packages": [
                {
                    "os": "linux",
                    "arch": "amd64",
                    "url": file_url,
                    "sha256": "f7a5a8c16a94fe934007f777a1bf532ef7e42b02133e31abf7523177b220a1ce"
                },
                {
                    "os": "macos",
                    "arch": "aarch64",
                    "url": file_url,
                    "sha256": "f7a5a8c16a94fe934007f777a1bf532ef7e42b02133e31abf7523177b220a1ce"
                },
                {
                    "os": "macos",
                    "arch": "amd64",
                    "url": file_url,
                    "sha256": "f7a5a8c16a94fe934007f777a1bf532ef7e42b02133e31abf7523177b220a1ce"
                }
            ]
        });
        let contents = serde_json::to_string(&plugin_manifest_json).unwrap();
        env.write_file("example-plugin-manifest.json", contents)?;

        // Install plugin
        let mut install = std::process::Command::new(spin_binary());
        install
            .args([
                "plugins",
                "install",
                "--file",
                "example-plugin-manifest.json",
                "--yes",
            ])
            // Ensure that spin installs the plugins into the temporary directory
            .env("TEST_PLUGINS_DIRECTORY", "./plugins");
        env.run_in(&mut install)?;

        /// Make sure that the plugin is uninstalled after the test
        struct Uninstaller<'a>(&'a testing_framework::TestEnvironment<()>);
        impl<'a> Drop for Uninstaller<'a> {
            fn drop(&mut self) {
                let mut uninstall = std::process::Command::new(spin_binary());
                uninstall.args(["plugins", "uninstall", "example"]);
                self.0.run_in(&mut uninstall).unwrap();
            }
        }
        let _u = Uninstaller(&env);

        let mut install = std::process::Command::new(spin_binary());
        install
            .args([
                "plugins",
                "install",
                "--file",
                "example-plugin-manifest.json",
                "--yes",
            ])
            // Ensure that spin installs the plugins into the temporary directory
            .env("TEST_PLUGINS_DIRECTORY", "./plugins");
        env.run_in(&mut install)?;

        let mut execute = std::process::Command::new(spin_binary());
        execute
            .args(["example"])
            .env("TEST_PLUGINS_DIRECTORY", "./plugins");
        let output = env.run_in(&mut execute)?;

        // Verify plugin successfully wrote to output file
        assert!(std::str::from_utf8(&output.stdout)?
            .trim()
            .contains("This is an example Spin plugin!"));

        // Upgrade plugin to newer version
        *plugin_manifest_json.get_mut("version").unwrap() = serde_json::json!("0.2.1");
        env.write_file(
            "example-plugin-manifest.json",
            serde_json::to_string(&plugin_manifest_json).unwrap(),
        )?;
        let mut upgrade = std::process::Command::new(spin_binary());
        upgrade
            .args([
                "plugins",
                "upgrade",
                "example",
                "--file",
                "example-plugin-manifest.json",
                "--yes",
            ])
            .env("TEST_PLUGINS_DIRECTORY", "./plugins");
        env.run_in(&mut upgrade)?;

        // Check plugin version
        let installed_manifest = std::path::PathBuf::from("plugins")
            .join("spin")
            .join("plugins")
            .join("manifests")
            .join("example.json");
        let manifest = String::from_utf8(env.read_file(installed_manifest)?).unwrap();
        assert!(manifest.contains("0.2.1"));

        Ok(())
    }

    // TODO: Test on Windows
    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_cloud_plugin_autoinstall() -> anyhow::Result<()> {
        let env = testing_framework::TestEnvironment::<()>::boot(
            &testing_framework::ServicesConfig::none(),
        )?;

        let mut login = std::process::Command::new(spin_binary());
        login
            .args(["login", "--help"])
            // Ensure that spin installs the plugins into the temporary directory
            .env("TEST_PLUGINS_DIRECTORY", "./plugins");
        let output = env.run_in(&mut login)?;

        // Verify plugin successfully wrote to output file
        assert!(std::str::from_utf8(&output.stdout)?
            .trim()
            .contains("The `cloud` plugin is required. Installing now."));
        // Ensure login help info is displayed
        assert!(std::str::from_utf8(&output.stdout)?
            .trim()
            .contains("Log into Fermyon Cloud"));
        Ok(())
    }

    #[test]
    fn test_build_command() -> anyhow::Result<()> {
        do_test_build_command("tests/testcases/simple-build")
    }

    /// Build an app whose component `workdir` is a subdirectory.
    #[test]
    #[cfg(not(tarpaulin))]
    fn test_build_command_nested_workdir() -> anyhow::Result<()> {
        do_test_build_command("tests/testcases/nested-build")
    }

    /// Builds app in `dir` and verifies the build succeeded. Expects manifest
    /// in `spin.toml` inside `dir`.
    fn do_test_build_command(dir: impl AsRef<std::path::Path>) -> anyhow::Result<()> {
        let dir = dir.as_ref();
        let manifest_file = dir.join("spin.toml");
        let manifest = spin_manifest::manifest_from_file(manifest_file)?;

        let sources = manifest
            .components
            .iter()
            .map(|(id, component)| {
                let spin_manifest::schema::v2::ComponentSource::Local(file) = &component.source
                else {
                    panic!(
                        "{}.{}: source is not a file reference",
                        manifest.application.name, id
                    )
                };
                (id, std::path::PathBuf::from(file))
            })
            .collect::<std::collections::HashMap<_, _>>();

        let env = testing_framework::TestEnvironment::<()>::boot(
            &testing_framework::ServicesConfig::none(),
        )?;
        env.copy_into(dir, ".")?;

        let mut build = std::process::Command::new(spin_binary());
        build.arg("build");
        env.run_in(&mut build)?;

        let mut missing_sources_count = 0;
        for (component_id, source) in sources.iter() {
            if env.read_file(source).is_err() {
                missing_sources_count += 1;
                println!(
                    "{}.{} source file '{}' was not generated by build",
                    manifest.application.name,
                    component_id,
                    source.display()
                );
            }
        }
        assert_eq!(missing_sources_count, 0);

        Ok(())
    }
}
