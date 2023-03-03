#[cfg(feature = "e2e-tests")]
pub mod all {
    use anyhow::Result;
    use e2e_testing::controller::Controller;
    use e2e_testing::http_asserts::assert_http_response;
    use e2e_testing::metadata_extractor::AppMetadata;
    use e2e_testing::spin;
    use e2e_testing::testcase::TestCaseBuilder;
    use e2e_testing::utils;
    use hyper::Method;
    use std::time::Duration;
    use tokio::io::BufReader;
    use tokio::process::{ChildStderr, ChildStdout};
    use tokio::time::sleep;

    fn get_url(base: &str, path: &str) -> String {
        format!("{}{}", base, path)
    }

    pub async fn key_value_works(controller: &dyn Controller) {
        async fn checks(
            metadata: AppMetadata,
            // TODO: investigate why omitting these two next parameters does not
            // cause a compile time error but causes a runtime one
            _: Option<BufReader<ChildStdout>>,
            _: Option<BufReader<ChildStderr>>,
        ) -> Result<()> {
            assert_http_response(
                get_url(metadata.base.as_str(), "/test").as_str(),
                Method::GET,
                "",
                200,
                &[],
                None,
            )
            .await
        }

        let tc = TestCaseBuilder::default()
            .name("key-value".to_string())
            .appname(Some("key-value".to_string()))
            .template(None)
            .assertions(
                |metadata: AppMetadata,
                 stdout_stream: Option<BufReader<ChildStdout>>,
                 stderr_stream: Option<BufReader<ChildStderr>>| {
                    Box::pin(checks(metadata, stdout_stream, stderr_stream))
                },
            )
            .build()
            .unwrap();

        tc.run(controller).await.unwrap()
    }

    pub async fn http_python_works(controller: &dyn Controller) {
        async fn checks(
            metadata: AppMetadata,
            _: Option<BufReader<ChildStdout>>,
            _: Option<BufReader<ChildStderr>>,
        ) -> Result<()> {
            assert_http_response(
                metadata.base.as_str(),
                Method::GET,
                "",
                200,
                &[],
                Some("Hello from the Python SDK"),
            )
            .await
        }

        let tc = TestCaseBuilder::default()
            .name("http-py-template".to_string())
            .template(Some("http-py".to_string()))
            .template_install_args(Some(vec![
                "--git".to_string(),
                "https://github.com/fermyon/spin-python-sdk".to_string(),
                "--update".to_string(),
            ]))
            .plugins(Some(vec!["py2wasm".to_string()]))
            .assertions(
                |metadata: AppMetadata,
                 stdout_stream: Option<BufReader<ChildStdout>>,
                 stderr_stream: Option<BufReader<ChildStderr>>| {
                    Box::pin(checks(metadata, stdout_stream, stderr_stream))
                },
            )
            .build()
            .unwrap();

        tc.run(controller).await.unwrap()
    }

    pub async fn http_php_works(controller: &dyn Controller) {
        async fn checks(
            metadata: AppMetadata,
            _: Option<BufReader<ChildStdout>>,
            _: Option<BufReader<ChildStderr>>,
        ) -> Result<()> {
            assert_http_response(
                metadata.base.as_str(),
                Method::GET,
                "",
                200,
                &[],
                Some("Hello Fermyon Spin\n"),
            )
            .await
        }

        let tc = TestCaseBuilder::default()
            .name("http-php-template".to_string())
            .template(Some("http-php".to_string()))
            .assertions(
                |metadata: AppMetadata,
                 stdout_stream: Option<BufReader<ChildStdout>>,
                 stderr_stream: Option<BufReader<ChildStderr>>| {
                    Box::pin(checks(metadata, stdout_stream, stderr_stream))
                },
            )
            .build()
            .unwrap();

        tc.run(controller).await.unwrap();
    }

    pub async fn http_swift_works(controller: &dyn Controller) {
        async fn checks(
            metadata: AppMetadata,
            _: Option<BufReader<ChildStdout>>,
            _: Option<BufReader<ChildStderr>>,
        ) -> Result<()> {
            assert_http_response(
                metadata.base.as_str(),
                Method::GET,
                "",
                200,
                &[],
                Some("Hello from WAGI/1!\n"),
            )
            .await
        }

        let tc = TestCaseBuilder::default()
            .name("http-swift-template".to_string())
            .template(Some("http-swift".to_string()))
            .assertions(
                |metadata: AppMetadata,
                 stdout_stream: Option<BufReader<ChildStdout>>,
                 stderr_stream: Option<BufReader<ChildStderr>>| {
                    Box::pin(checks(metadata, stdout_stream, stderr_stream))
                },
            )
            .build()
            .unwrap();

        tc.run(controller).await.unwrap();
    }

    pub async fn http_go_works(controller: &dyn Controller) {
        async fn checks(
            metadata: AppMetadata,
            _: Option<BufReader<ChildStdout>>,
            _: Option<BufReader<ChildStderr>>,
        ) -> Result<()> {
            assert_http_response(
                metadata.base.as_str(),
                Method::GET,
                "",
                200,
                &[],
                Some("Hello Fermyon!\n"),
            )
            .await
        }

        let tc = TestCaseBuilder::default()
            .name("http-go-template".to_string())
            .template(Some("http-go".to_string()))
            .assertions(
                |metadata: AppMetadata,
                 stdout_stream: Option<BufReader<ChildStdout>>,
                 stderr_stream: Option<BufReader<ChildStderr>>| {
                    Box::pin(checks(metadata, stdout_stream, stderr_stream))
                },
            )
            .build()
            .unwrap();

        tc.run(controller).await.unwrap();
    }

    pub async fn http_c_works(controller: &dyn Controller) {
        async fn checks(
            metadata: AppMetadata,
            _: Option<BufReader<ChildStdout>>,
            _: Option<BufReader<ChildStderr>>,
        ) -> Result<()> {
            assert_http_response(
                metadata.base.as_str(),
                Method::GET,
                "",
                200,
                &[],
                Some("Hello from WAGI/1\n"),
            )
            .await
        }

        let tc = TestCaseBuilder::default()
            .name("http-c-template".to_string())
            .template(Some("http-c".to_string()))
            .assertions(
                |metadata: AppMetadata,
                 stdout_stream: Option<BufReader<ChildStdout>>,
                 stderr_stream: Option<BufReader<ChildStderr>>| {
                    Box::pin(checks(metadata, stdout_stream, stderr_stream))
                },
            )
            .build()
            .unwrap();

        tc.run(controller).await.unwrap()
    }

    pub async fn http_rust_works(controller: &dyn Controller) {
        async fn checks(
            metadata: AppMetadata,
            _: Option<BufReader<ChildStdout>>,
            _: Option<BufReader<ChildStderr>>,
        ) -> Result<()> {
            assert_http_response(
                metadata.base.as_str(),
                Method::GET,
                "",
                200,
                &[],
                Some("Hello, Fermyon"),
            )
            .await
        }

        let tc = TestCaseBuilder::default()
            .name("http-rust-template".to_string())
            .template(Some("http-rust".to_string()))
            .assertions(
                |metadata: AppMetadata,
                 stdout_stream: Option<BufReader<ChildStdout>>,
                 stderr_stream: Option<BufReader<ChildStderr>>| {
                    Box::pin(checks(metadata, stdout_stream, stderr_stream))
                },
            )
            .build()
            .unwrap();

        tc.run(controller).await.unwrap()
    }

    pub async fn http_zig_works(controller: &dyn Controller) {
        async fn checks(
            metadata: AppMetadata,
            _: Option<BufReader<ChildStdout>>,
            _: Option<BufReader<ChildStderr>>,
        ) -> Result<()> {
            assert_http_response(
                metadata.base.as_str(),
                Method::GET,
                "",
                200,
                &[],
                Some("Hello World!\n"),
            )
            .await
        }

        let tc = TestCaseBuilder::default()
            .name("http-zig-template".to_string())
            .template(Some("http-zig".to_string()))
            .assertions(
                |metadata: AppMetadata,
                 stdout_stream: Option<BufReader<ChildStdout>>,
                 stderr_stream: Option<BufReader<ChildStderr>>| {
                    Box::pin(checks(metadata, stdout_stream, stderr_stream))
                },
            )
            .build()
            .unwrap();

        tc.run(controller).await.unwrap()
    }

    #[allow(unused)]
    pub async fn http_grain_works(controller: &dyn Controller) {
        async fn checks(
            metadata: AppMetadata,
            _: Option<BufReader<ChildStdout>>,
            _: Option<BufReader<ChildStderr>>,
        ) -> Result<()> {
            assert_http_response(
                metadata.base.as_str(),
                Method::GET,
                "",
                200,
                &[],
                Some("Hello, World\n"),
            )
            .await
        }

        let tc = TestCaseBuilder::default()
            .name("http-grain-template".to_string())
            .template(Some("http-grain".to_string()))
            .assertions(
                |metadata: AppMetadata,
                 stdout_stream: Option<BufReader<ChildStdout>>,
                 stderr_stream: Option<BufReader<ChildStderr>>| {
                    Box::pin(checks(metadata, stdout_stream, stderr_stream))
                },
            )
            .build()
            .unwrap();

        tc.run(controller).await.unwrap()
    }

    pub async fn http_ts_works(controller: &dyn Controller) {
        async fn checks(
            metadata: AppMetadata,
            _: Option<BufReader<ChildStdout>>,
            _: Option<BufReader<ChildStderr>>,
        ) -> Result<()> {
            assert_http_response(
                metadata.base.as_str(),
                Method::GET,
                "",
                200,
                &[],
                Some("Hello from TS-SDK"),
            )
            .await
        }

        let tc = TestCaseBuilder::default()
            .name("http-ts-template".to_string())
            .template(Some("http-ts".to_string()))
            .template_install_args(Some(vec![
                "--git".to_string(),
                "https://github.com/fermyon/spin-js-sdk".to_string(),
                "--update".to_string(),
            ]))
            .plugins(Some(vec!["js2wasm".to_string()]))
            .pre_build_hooks(Some(vec![vec!["npm".to_string(), "install".to_string()]]))
            .assertions(
                |metadata: AppMetadata,
                 stdout_stream: Option<BufReader<ChildStdout>>,
                 stderr_stream: Option<BufReader<ChildStderr>>| {
                    Box::pin(checks(metadata, stdout_stream, stderr_stream))
                },
            )
            .build()
            .unwrap();

        tc.run(controller).await.unwrap()
    }

    pub async fn http_js_works(controller: &dyn Controller) {
        async fn checks(
            metadata: AppMetadata,
            _: Option<BufReader<ChildStdout>>,
            _: Option<BufReader<ChildStderr>>,
        ) -> Result<()> {
            assert_http_response(
                metadata.base.as_str(),
                Method::GET,
                "",
                200,
                &[],
                Some("Hello from JS-SDK"),
            )
            .await
        }

        let tc = TestCaseBuilder::default()
            .name("http-js-template".to_string())
            .template(Some("http-js".to_string()))
            .template_install_args(Some(vec![
                "--git".to_string(),
                "https://github.com/fermyon/spin-js-sdk".to_string(),
                "--update".to_string(),
            ]))
            .plugins(Some(vec!["js2wasm".to_string()]))
            .pre_build_hooks(Some(vec![vec!["npm".to_string(), "install".to_string()]]))
            .assertions(
                |metadata: AppMetadata,
                 stdout_stream: Option<BufReader<ChildStdout>>,
                 stderr_stream: Option<BufReader<ChildStderr>>| {
                    Box::pin(checks(metadata, stdout_stream, stderr_stream))
                },
            )
            .build()
            .unwrap();

        tc.run(controller).await.unwrap()
    }

    pub async fn assets_routing_works(controller: &dyn Controller) {
        async fn checks(
            metadata: AppMetadata,
            _: Option<BufReader<ChildStdout>>,
            _: Option<BufReader<ChildStderr>>,
        ) -> Result<()> {
            assert_http_response(
                get_url(metadata.base.as_str(), "/static/thisshouldbemounted/1").as_str(),
                Method::GET,
                "",
                200,
                &[],
                Some("1\n"),
            )
            .await?;

            assert_http_response(
                get_url(metadata.base.as_str(), "/static/thisshouldbemounted/2").as_str(),
                Method::GET,
                "",
                200,
                &[],
                Some("2\n"),
            )
            .await?;

            assert_http_response(
                get_url(metadata.base.as_str(), "/static/thisshouldbemounted/3").as_str(),
                Method::GET,
                "",
                200,
                &[],
                Some("3\n"),
            )
            .await?;

            assert_http_response(
                get_url(metadata.base.as_str(), "/static/donotmount/a").as_str(),
                Method::GET,
                "",
                404,
                &[],
                Some("Not Found"),
            )
            .await?;

            assert_http_response(
                get_url(
                    metadata.base.as_str(),
                    "/static/thisshouldbemounted/thisshouldbeexcluded/4",
                )
                .as_str(),
                Method::GET,
                "",
                404,
                &[],
                Some("Not Found"),
            )
            .await?;

            Ok(())
        }

        let tc = TestCaseBuilder::default()
            .name("assets-test".to_string())
            .appname(Some("assets-test".to_string()))
            .assertions(
                |metadata: AppMetadata,
                 stdout_stream: Option<BufReader<ChildStdout>>,
                 stderr_stream: Option<BufReader<ChildStderr>>| {
                    Box::pin(checks(metadata, stdout_stream, stderr_stream))
                },
            )
            .build()
            .unwrap();

        tc.run(controller).await.unwrap()
    }

    pub async fn simple_spin_rust_works(controller: &dyn Controller) {
        async fn checks(
            metadata: AppMetadata,
            _: Option<BufReader<ChildStdout>>,
            _: Option<BufReader<ChildStderr>>,
        ) -> Result<()> {
            assert_http_response(
                get_url(metadata.base.as_str(), "/test/hello").as_str(),
                Method::GET,
                "",
                200,
                &[],
                Some("I'm a teapot"),
            )
            .await?;

            assert_http_response(
                get_url(
                    metadata.base.as_str(),
                    "/test/hello/wildcards/should/be/handled",
                )
                .as_str(),
                Method::GET,
                "",
                200,
                &[],
                Some("I'm a teapot"),
            )
            .await?;

            assert_http_response(
                get_url(metadata.base.as_str(), "/thisshouldfail").as_str(),
                Method::GET,
                "",
                404,
                &[],
                None,
            )
            .await?;

            assert_http_response(
                get_url(metadata.base.as_str(), "/test/hello/test-placement").as_str(),
                Method::GET,
                "",
                200,
                &[],
                Some("text for test"),
            )
            .await?;

            Ok(())
        }

        let tc = TestCaseBuilder::default()
            .name("simple-spin-rust-test".to_string())
            .appname(Some("simple-spin-rust-test".to_string()))
            .assertions(
                |metadata: AppMetadata,
                 stdout_stream: Option<BufReader<ChildStdout>>,
                 stderr_stream: Option<BufReader<ChildStderr>>| {
                    Box::pin(checks(metadata, stdout_stream, stderr_stream))
                },
            )
            .build()
            .unwrap();

        tc.run(controller).await.unwrap()
    }

    pub async fn header_env_routes_works(controller: &dyn Controller) {
        async fn checks(
            metadata: AppMetadata,
            _: Option<BufReader<ChildStdout>>,
            _: Option<BufReader<ChildStderr>>,
        ) -> Result<()> {
            assert_http_response(
                get_url(metadata.base.as_str(), "/env").as_str(),
                Method::GET,
                "",
                200,
                &[],
                Some("I'm a teapot"),
            )
            .await?;

            assert_http_response(
                get_url(metadata.base.as_str(), "/env/foo").as_str(),
                Method::GET,
                "",
                200,
                &[("env_some_key", "some_value")],
                Some("I'm a teapot"),
            )
            .await?;

            Ok(())
        }

        let tc = TestCaseBuilder::default()
            .name("headers-env-routes-test".to_string())
            .appname(Some("headers-env-routes-test".to_string()))
            .assertions(
                |metadata: AppMetadata,
                 stdout_stream: Option<BufReader<ChildStdout>>,
                 stderr_stream: Option<BufReader<ChildStderr>>| {
                    Box::pin(checks(metadata, stdout_stream, stderr_stream))
                },
            )
            .build()
            .unwrap();

        tc.run(controller).await.unwrap()
    }

    pub async fn header_dynamic_env_works(controller: &dyn Controller) {
        async fn checks(
            metadata: AppMetadata,
            _: Option<BufReader<ChildStdout>>,
            _: Option<BufReader<ChildStderr>>,
        ) -> Result<()> {
            assert_http_response(
                get_url(metadata.base.as_str(), "/env").as_str(),
                Method::GET,
                "",
                200,
                &[],
                Some("I'm a teapot"),
            )
            .await?;

            assert_http_response(
                get_url(metadata.base.as_str(), "/env/foo").as_str(),
                Method::GET,
                "",
                200,
                &[("env_some_key", "some_value")],
                Some("I'm a teapot"),
            )
            .await?;

            Ok(())
        }

        let tc = TestCaseBuilder::default()
            .name("headers-dynamic-env-test".to_string())
            .appname(Some("headers-dynamic-env-test".to_string()))
            .deploy_args(vec!["--env".to_string(), "foo=bar".to_string()])
            .assertions(
                |metadata: AppMetadata,
                 stdout_stream: Option<BufReader<ChildStdout>>,
                 stderr_stream: Option<BufReader<ChildStderr>>| {
                    Box::pin(checks(metadata, stdout_stream, stderr_stream))
                },
            )
            .build()
            .unwrap();

        tc.run(controller).await.unwrap()
    }

    pub async fn http_rust_outbound_mysql_works(controller: &dyn Controller) {
        async fn checks(
            metadata: AppMetadata,
            _: Option<BufReader<ChildStdout>>,
            _: Option<BufReader<ChildStderr>>,
        ) -> Result<()> {
            assert_http_response(
                get_url(metadata.base.as_str(), "/test_numeric_types").as_str(),
                Method::GET,
                "",
                200,
                &[],
                None,
            )
            .await?;

            assert_http_response(
                get_url(metadata.base.as_str(), "/test_character_types").as_str(),
                Method::GET,
                "",
                200,
                &[],
                None,
            )
            .await?;

            Ok(())
        }

        let tc = TestCaseBuilder::default()
            .name("http-rust-outbound-mysql".to_string())
            .appname(Some("http-rust-outbound-mysql".to_string()))
            .deploy_args(vec!["--env".to_string(), "foo=bar".to_string()])
            .assertions(
                |metadata: AppMetadata,
                 stdout_stream: Option<BufReader<ChildStdout>>,
                 stderr_stream: Option<BufReader<ChildStderr>>| {
                    Box::pin(checks(metadata, stdout_stream, stderr_stream))
                },
            )
            .build()
            .unwrap();

        tc.run(controller).await.unwrap()
    }

    pub async fn redis_go_works(controller: &dyn Controller) {
        async fn checks(
            _: AppMetadata,
            _: Option<BufReader<ChildStdout>>,
            stderr_stream: Option<BufReader<ChildStderr>>,
        ) -> Result<()> {
            //TODO: wait for spin up to be ready dynamically
            sleep(Duration::from_secs(10)).await;

            utils::run(
                vec![
                    "redis-cli",
                    "-u",
                    "redis://redis:6379",
                    "PUBLISH",
                    "redis-go-works-channel",
                    "msg-from-go-channel",
                ],
                None,
                None,
            )?;

            let stderr =
                utils::get_output_from_stderr(stderr_stream, Duration::from_secs(5)).await?;
            let expected_logs = vec!["Payload::::", "msg-from-go-channel"];

            assert!(expected_logs
                .iter()
                .all(|item| stderr.contains(&item.to_string())));

            Ok(())
        }

        let tc = TestCaseBuilder::default()
            .name("redis-go".to_string())
            .template(Some("redis-go".to_string()))
            .new_app_args(vec![
                "--value".to_string(),
                "redis-channel=redis-go-works-channel".to_string(),
                "--value".to_string(),
                "redis-address=redis://redis:6379".to_string(),
            ])
            .trigger_type("redis".to_string())
            .assertions(
                |metadata: AppMetadata,
                 stdout_stream: Option<BufReader<ChildStdout>>,
                 stderr_stream: Option<BufReader<ChildStderr>>| {
                    Box::pin(checks(metadata, stdout_stream, stderr_stream))
                },
            )
            .build()
            .unwrap();

        tc.run(controller).await.unwrap()
    }

    pub async fn redis_rust_works(controller: &dyn Controller) {
        async fn checks(
            _: AppMetadata,
            _: Option<BufReader<ChildStdout>>,
            stderr_stream: Option<BufReader<ChildStderr>>,
        ) -> Result<()> {
            //TODO: wait for spin up to be ready dynamically
            sleep(Duration::from_secs(20)).await;

            utils::run(
                vec![
                    "redis-cli",
                    "-u",
                    "redis://redis:6379",
                    "PUBLISH",
                    "redis-rust-works-channel",
                    "msg-from-rust-channel",
                ],
                None,
                None,
            )?;

            let stderr =
                utils::get_output_from_stderr(stderr_stream, Duration::from_secs(5)).await?;

            let expected_logs = vec!["msg-from-rust-channel"];

            assert!(expected_logs
                .iter()
                .all(|item| stderr.contains(&item.to_string())));

            Ok(())
        }

        let tc = TestCaseBuilder::default()
            .name("redis-rust".to_string())
            .template(Some("redis-rust".to_string()))
            .new_app_args(vec![
                "--value".to_string(),
                "redis-channel=redis-rust-works-channel".to_string(),
                "--value".to_string(),
                "redis-address=redis://redis:6379".to_string(),
            ])
            .trigger_type("redis".to_string())
            .assertions(
                |metadata: AppMetadata,
                 stdout_stream: Option<BufReader<ChildStdout>>,
                 stderr_stream: Option<BufReader<ChildStderr>>| {
                    Box::pin(checks(metadata, stdout_stream, stderr_stream))
                },
            )
            .build()
            .unwrap();

        tc.run(controller).await.unwrap()
    }

    pub async fn http_rust_outbound_pg_works(controller: &dyn Controller) {
        async fn checks(
            metadata: AppMetadata,
            _: Option<BufReader<ChildStdout>>,
            _: Option<BufReader<ChildStderr>>,
        ) -> Result<()> {
            assert_http_response(
                get_url(metadata.base.as_str(), "/test_numeric_types").as_str(),
                Method::GET,
                "",
                200,
                &[],
                None,
            )
            .await?;

            assert_http_response(
                get_url(metadata.base.as_str(), "/test_character_types").as_str(),
                Method::GET,
                "",
                200,
                &[],
                None,
            )
            .await?;

            assert_http_response(
                get_url(metadata.base.as_str(), "/test_general_types").as_str(),
                Method::GET,
                "",
                200,
                &[],
                None,
            )
            .await?;

            assert_http_response(
                get_url(metadata.base.as_str(), "/pg_backend_pid").as_str(),
                Method::GET,
                "",
                200,
                &[],
                None,
            )
            .await?;

            Ok(())
        }

        let tc = TestCaseBuilder::default()
            .name("http-rust-outbound-pg".to_string())
            .appname(Some("http-rust-outbound-pg".to_string()))
            .assertions(
                |metadata: AppMetadata,
                 stdout_stream: Option<BufReader<ChildStdout>>,
                 stderr_stream: Option<BufReader<ChildStderr>>| {
                    Box::pin(checks(metadata, stdout_stream, stderr_stream))
                },
            )
            .build()
            .unwrap();

        tc.run(controller).await.unwrap()
    }

    pub async fn registry_works(controller: &dyn Controller) {
        async fn checks(
            metadata: AppMetadata,
            _: Option<BufReader<ChildStdout>>,
            _: Option<BufReader<ChildStderr>>,
        ) -> Result<()> {
            assert_http_response(
                metadata.base.as_str(),
                Method::GET,
                "",
                200,
                &[],
                Some("Hello Fermyon!\n"),
            )
            .await
        }

        let registry = "registry:5000";
        let registry_app_url = format!(
            "{}/{}/{}:{}",
            registry, "spin-e2e-tests", "registry_works", "v1"
        );
        let tc = TestCaseBuilder::default()
            .name("http-go".to_string())
            .template(Some("http-go".to_string()))
            .appname(Some("http-go-registry-generated".to_string()))
            .push_to_registry(Some(registry_app_url.clone()))
            .deploy_args(vec![
                "--from-registry".to_string(),
                registry_app_url.clone(),
                "--insecure".to_string(),
            ])
            .assertions(
                |metadata: AppMetadata,
                 stdout_stream: Option<BufReader<ChildStdout>>,
                 stderr_stream: Option<BufReader<ChildStderr>>| {
                    Box::pin(checks(metadata, stdout_stream, stderr_stream))
                },
            )
            .build()
            .unwrap();

        tc.run(controller).await.unwrap()
    }

    pub async fn longevity_apps_works(controller: &dyn Controller) {
        let current_spin_version = match spin::version() {
            Ok(version) => version,
            Err(err) => {
                println!("error getting spin version {}", err);
                "".to_string()
            }
        };

        // version of spin that was used to generate the wasm files used in this testcase
        const SPIN_VERSION_USED_TO_BUILD_APP: &str = "spin version 0.9.0 (a99ed51 2023-02-16)";

        async fn checks(
            metadata: AppMetadata,
            _: Option<BufReader<ChildStdout>>,
            _: Option<BufReader<ChildStderr>>,
        ) -> Result<()> {
            assert_http_response(
                get_url(metadata.base.as_str(), "/golang").as_str(),
                Method::GET,
                "",
                200,
                &[],
                Some("Hello Fermyon!\n"),
            )
            .await?;

            assert_http_response(
                get_url(metadata.base.as_str(), "/rust").as_str(),
                Method::GET,
                "",
                200,
                &[],
                Some("Hello, Fermyon"),
            )
            .await?;

            assert_http_response(
                get_url(metadata.base.as_str(), "/javascript").as_str(),
                Method::GET,
                "",
                200,
                &[],
                Some("Hello from JS-SDK"),
            )
            .await?;

            assert_http_response(
                get_url(metadata.base.as_str(), "/typescript").as_str(),
                Method::GET,
                "",
                200,
                &[],
                Some("Hello from TS-SDK"),
            )
            .await?;

            Ok(())
        }

        let tc = TestCaseBuilder::default()
            .name("longevity-apps-test".to_string())
            .appname(Some("longevity-apps-test".to_string()))
            .assertions(
                |metadata: AppMetadata,
                 stdout_stream: Option<BufReader<ChildStdout>>,
                 stderr_stream: Option<BufReader<ChildStderr>>| {
                    Box::pin(checks(metadata, stdout_stream, stderr_stream))
                },
            )
            .build()
            .unwrap();
        let msg = format!(
            "app using a wasm module built by '{}' no longer works with {}",
            SPIN_VERSION_USED_TO_BUILD_APP, current_spin_version
        );
        if let Err(e) = tc.run(controller).await {
            panic!(":\n\n{}\nError: {}", msg, e)
        }
    }
}
