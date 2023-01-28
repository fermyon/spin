#[cfg(feature = "new-e2e-tests")]
pub mod all {
    use anyhow::Result;
    use e2e_testing::asserts::assert_http_request;
    use e2e_testing::cloud_controller;
    use e2e_testing::controller::Controller;
    use e2e_testing::metadata_extractor::AppMetadata;
    use e2e_testing::testcase::{SkipCondition, TestCase};

    fn get_url(base: &str, path: &str) -> String {
        format!("{}{}", base, path)
    }

    pub async fn http_go_works(controller: &dyn Controller) {
        fn checks(metadata: &AppMetadata) -> Result<()> {
            return assert_http_request(metadata.base.as_str(), 200, &[], Some("Hello Fermyon!\n"));
        }

        let tc = TestCase {
            name: "http-go template".to_string(),
            appname: "http-go-test".to_string(),
            template: Some("http-go".to_string()),
            template_install_args: None,
            assertions: checks,
            plugins: None,
            deploy_args: None,
            skip_conditions: None,
            pre_build_hooks: None,
        };

        tc.run(controller).await.unwrap();
    }

    pub async fn http_c_works(controller: &dyn Controller) {
        fn checks(metadata: &AppMetadata) -> Result<()> {
            return assert_http_request(
                metadata.base.as_str(),
                200,
                &[],
                Some("Hello from WAGI/1\n"),
            );
        }

        let tc = TestCase {
            name: "http-c template".to_string(),
            appname: "http-c-test".to_string(),
            template: Some("http-c".to_string()),
            template_install_args: None,
            assertions: checks,
            plugins: None,
            deploy_args: None,
            skip_conditions: None,
            pre_build_hooks: None,
        };

        tc.run(controller).await.unwrap()
    }

    pub async fn http_rust_works(controller: &dyn Controller) {
        fn checks(metadata: &AppMetadata) -> Result<()> {
            return assert_http_request(metadata.base.as_str(), 200, &[], Some("Hello, Fermyon"));
        }

        let tc = TestCase {
            name: "http-rust-template".to_string(),
            appname: "http-rust-test".to_string(),
            template: Some("http-rust".to_string()),
            template_install_args: None,
            assertions: checks,
            plugins: None,
            deploy_args: None,
            skip_conditions: None,
            pre_build_hooks: None,
        };

        tc.run(controller).await.unwrap()
    }

    pub async fn http_zig_works(controller: &dyn Controller) {
        fn checks(metadata: &AppMetadata) -> Result<()> {
            return assert_http_request(metadata.base.as_str(), 200, &[], Some("Hello World!\n"));
        }

        let tc = TestCase {
            name: "http-zig-template".to_string(),
            appname: "http-zig-test".to_string(),
            template: Some("http-zig".to_string()),
            template_install_args: None,
            assertions: checks,
            plugins: None,
            deploy_args: None,
            skip_conditions: None,
            pre_build_hooks: None,
        };

        tc.run(controller).await.unwrap()
    }

    pub async fn http_grain_works(controller: &dyn Controller) {
        fn checks(metadata: &AppMetadata) -> Result<()> {
            return assert_http_request(metadata.base.as_str(), 200, &[], Some("Hello, World\n"));
        }

        let tc = TestCase {
            name: "http-grain-template".to_string(),
            appname: "http-grain-test".to_string(),
            template: Some("http-grain".to_string()),
            template_install_args: None,
            assertions: checks,
            plugins: None,
            deploy_args: None,
            skip_conditions: None,
            pre_build_hooks: None,
        };

        tc.run(controller).await.unwrap()
    }

    pub async fn http_ts_works(controller: &dyn Controller) {
        fn checks(metadata: &AppMetadata) -> Result<()> {
            return assert_http_request(
                metadata.base.as_str(),
                200,
                &[],
                Some("Hello from TS-SDK"),
            );
        }

        let tc = TestCase {
            name: "http-ts-template".to_string(),
            appname: "http-ts-test".to_string(),
            template: Some("http-ts".to_string()),
            template_install_args: Some(vec![
                "--git".to_string(),
                "https://github.com/fermyon/spin-js-sdk".to_string(),
                "--update".to_string(),
            ]),
            assertions: checks,
            plugins: Some(vec!["js2wasm".to_string()]),
            deploy_args: None,
            skip_conditions: None,
            pre_build_hooks: Some(vec![vec!["npm".to_string(), "install".to_string()]]),
        };

        tc.run(controller).await.unwrap()
    }

    pub async fn http_js_works(controller: &dyn Controller) {
        fn checks(metadata: &AppMetadata) -> Result<()> {
            return assert_http_request(
                metadata.base.as_str(),
                200,
                &[],
                Some("Hello from JS-SDK"),
            );
        }

        let tc = TestCase {
            name: "http-js-template".to_string(),
            appname: "http-js-test".to_string(),
            template: Some("http-js".to_string()),
            template_install_args: Some(vec![
                "--git".to_string(),
                "https://github.com/fermyon/spin-js-sdk".to_string(),
                "--update".to_string(),
            ]),
            assertions: checks,
            plugins: Some(vec!["js2wasm".to_string()]),
            deploy_args: None,
            skip_conditions: None,
            pre_build_hooks: Some(vec![vec!["npm".to_string(), "install".to_string()]]),
        };

        tc.run(controller).await.unwrap()
    }

    pub async fn assets_routing_works(controller: &dyn Controller) {
        fn checks(metadata: &AppMetadata) -> Result<()> {
            assert_http_request(
                get_url(metadata.base.as_str(), "/static/thisshouldbemounted/1").as_str(),
                200,
                &[],
                Some("1\n"),
            )?;

            assert_http_request(
                get_url(metadata.base.as_str(), "/static/thisshouldbemounted/2").as_str(),
                200,
                &[],
                Some("2\n"),
            )?;

            assert_http_request(
                get_url(metadata.base.as_str(), "/static/thisshouldbemounted/3").as_str(),
                200,
                &[],
                Some("3\n"),
            )?;

            assert_http_request(
                get_url(metadata.base.as_str(), "/static/donotmount/a").as_str(),
                404,
                &[],
                Some("Not Found"),
            )?;

            assert_http_request(
                get_url(
                    metadata.base.as_str(),
                    "/static/thisshouldbemounted/thisshouldbeexcluded/4",
                )
                .as_str(),
                404,
                &[],
                Some("Not Found"),
            )?;

            Ok(())
        }

        let tc = TestCase {
            name: "assets-test".to_string(),
            appname: "assets-test".to_string(),
            template: None,
            template_install_args: None,
            assertions: checks,
            plugins: None,
            deploy_args: None,
            skip_conditions: None,
            pre_build_hooks: None,
        };

        tc.run(controller).await.unwrap()
    }

    pub async fn simple_spin_rust_works(controller: &dyn Controller) {
        fn checks(metadata: &AppMetadata) -> Result<()> {
            assert_http_request(
                get_url(metadata.base.as_str(), "/test/hello").as_str(),
                200,
                &[],
                Some("I'm a teapot"),
            )?;

            assert_http_request(
                get_url(
                    metadata.base.as_str(),
                    "/test/hello/wildcards/should/be/handled",
                )
                .as_str(),
                200,
                &[],
                Some("I'm a teapot"),
            )?;

            assert_http_request(
                get_url(metadata.base.as_str(), "/thisshouldfail").as_str(),
                404,
                &[],
                None,
            )?;

            assert_http_request(
                get_url(metadata.base.as_str(), "/test/hello/test-placement").as_str(),
                200,
                &[],
                Some("text for test"),
            )?;

            Ok(())
        }

        let tc = TestCase {
            name: "simple-spin-rust-test".to_string(),
            appname: "simple-spin-rust-test".to_string(),
            template: None,
            template_install_args: None,
            assertions: checks,
            plugins: None,
            deploy_args: None,
            skip_conditions: None,
            pre_build_hooks: None,
        };

        tc.run(controller).await.unwrap()
    }

    pub async fn header_env_routes_works(controller: &dyn Controller) {
        fn checks(metadata: &AppMetadata) -> Result<()> {
            assert_http_request(
                get_url(metadata.base.as_str(), "/env").as_str(),
                200,
                &[],
                Some("I'm a teapot"),
            )?;

            assert_http_request(
                get_url(metadata.base.as_str(), "/env/foo").as_str(),
                200,
                &[("env_some_key", "some_value")],
                Some("I'm a teapot"),
            )?;

            Ok(())
        }

        let tc = TestCase {
            name: "headers-env-routes-test".to_string(),
            appname: "headers-env-routes-test".to_string(),
            template: None,
            template_install_args: None,
            assertions: checks,
            plugins: None,
            deploy_args: None,
            skip_conditions: None,
            pre_build_hooks: None,
        };

        tc.run(controller).await.unwrap()
    }

    pub async fn header_dynamic_env_works(controller: &dyn Controller) {
        fn checks(metadata: &AppMetadata) -> Result<()> {
            assert_http_request(
                get_url(metadata.base.as_str(), "/env").as_str(),
                200,
                &[],
                Some("I'm a teapot"),
            )?;

            assert_http_request(
                get_url(metadata.base.as_str(), "/env/foo").as_str(),
                200,
                &[("env_some_key", "some_value")],
                Some("I'm a teapot"),
            )?;

            Ok(())
        }

        let tc = TestCase {
            name: "headers-dynamic-env-test".to_string(),
            appname: "headers-dynamic-env-test".to_string(),
            template: None,
            template_install_args: None,
            assertions: checks,
            plugins: None,
            deploy_args: Some(vec!["--env".to_string(), "foo=bar".to_string()]),
            skip_conditions: Some(vec![SkipCondition {
                env: cloud_controller::NAME.to_string(),
                reason: "--env is not supported with Fermyon cloud".to_string(),
            }]),
            pre_build_hooks: None,
        };

        tc.run(controller).await.unwrap()
    }

    pub async fn http_rust_outbound_mysql_works(controller: &dyn Controller) {
        fn checks(metadata: &AppMetadata) -> Result<()> {
            assert_http_request(
                get_url(metadata.base.as_str(), "/test_numeric_types").as_str(),
                200,
                &[],
                None,
            )?;

            assert_http_request(
                get_url(metadata.base.as_str(), "/test_character_types").as_str(),
                200,
                &[],
                None,
            )?;

            Ok(())
        }

        let tc = TestCase {
            name: "http-rust-outbound-mysql".to_string(),
            appname: "http-rust-outbound-mysql".to_string(),
            template: None,
            template_install_args: None,
            assertions: checks,
            plugins: None,
            deploy_args: None,
            skip_conditions: None,
            pre_build_hooks: None,
        };

        tc.run(controller).await.unwrap()
    }
}
