use anyhow::Result;
use e2e_testing::controller::Controller;
use e2e_testing::http_asserts::assert_http_response;
use e2e_testing::metadata_extractor::AppMetadata;
use e2e_testing::testcase::TestCaseBuilder;
use e2e_testing::{spin, utils};
use hyper::Method;
use std::pin::Pin;
use std::time::Duration;
use tokio::io::AsyncBufRead;
use tokio::time::sleep;

fn get_url(base: &str, path: &str) -> String {
    format!("{}{}", base, path)
}

pub async fn config_variables_default_works(controller: &dyn Controller) {
    async fn checks(
        metadata: AppMetadata,
        password: String,
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
    ) -> Result<()> {
        assert_http_response(
            get_url(
                metadata.base.as_str(),
                &format!("/test?password={password}"),
            )
            .as_str(),
            Method::GET,
            "",
            200,
            &[],
            None,
        )
        .await
    }

    // Set to Spin.toml default: `password = { default = "pw" }`
    let expected_password = String::from("pw");

    let tc = TestCaseBuilder::default()
        .name("config-variables".to_string())
        .appname(Some("config-variables".to_string()))
        .template(None)
        .assertions(
            move |metadata: AppMetadata,
                  stdout_stream: Option<Pin<Box<dyn AsyncBufRead>>>,
                  stderr_stream: Option<Pin<Box<dyn AsyncBufRead>>>| {
                let pw = expected_password.clone();
                Box::pin(checks(metadata, pw, stdout_stream, stderr_stream))
            },
        )
        .build()
        .unwrap();

    tc.run(controller).await.unwrap()
}

pub async fn key_value_works(controller: &dyn Controller) {
    async fn checks(
        metadata: AppMetadata,
        test_init_key: String,
        test_init_value: String,
        // TODO: investigate why omitting these two next parameters does not
        // cause a compile time error but causes a runtime one
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
    ) -> Result<()> {
        assert_http_response(
            get_url(
                metadata.base.as_str(),
                &format!("/test?testkey={test_init_key}&testval={test_init_value}"),
            )
            .as_str(),
            Method::GET,
            "",
            200,
            &[],
            None,
        )
        .await
    }

    let init_key = uuid::Uuid::new_v4().to_string();
    let init_value = uuid::Uuid::new_v4().to_string();

    let tc = TestCaseBuilder::default()
        .name("key-value".to_string())
        .appname(Some("key-value".to_string()))
        .template(None)
        .deploy_args(vec![
            "--key-value".to_string(),
            format!("{init_key}={init_value}"),
        ])
        .assertions(
            move |metadata: AppMetadata,
                  stdout_stream: Option<Pin<Box<dyn AsyncBufRead>>>,
                  stderr_stream: Option<Pin<Box<dyn AsyncBufRead>>>| {
                let ik = init_key.clone();
                let iv = init_value.clone();
                Box::pin(checks(metadata, ik, iv, stdout_stream, stderr_stream))
            },
        )
        .build()
        .unwrap();

    tc.run(controller).await.unwrap()
}

pub async fn key_value_validation_works(controller: &dyn Controller) {
    async fn checks(
        _: AppMetadata,
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
        stderr: Option<Pin<Box<dyn AsyncBufRead>>>,
    ) -> Result<()> {
        let err_text = utils::get_output(stderr).await.unwrap();
        let expected1 = "Component hello uses store 'anaspeptic'";
        let expected2 = "Component hello uses store 'pericombobulations'";

        assert!(
            err_text.contains(expected1),
            "Expected error containing '{expected1}' but got '{err_text}'"
        );
        assert!(
            err_text.contains(expected2),
            "Expected error containing '{expected2}' but got '{err_text}'"
        );
        Ok(())
    }

    let tc = TestCaseBuilder::default()
        .name("key-value-undefined-store".to_string())
        .appname(Some("key-value-undefined-store".to_string()))
        .template(None)
        .assertions(
            |metadata: AppMetadata,
             stdout_stream: Option<Pin<Box<dyn AsyncBufRead>>>,
             stderr_stream: Option<Pin<Box<dyn AsyncBufRead>>>| {
                Box::pin(checks(metadata, stdout_stream, stderr_stream))
            },
        )
        .build()
        .unwrap();

    tc.try_run(controller).await.unwrap();
}

pub async fn sqlite_works(controller: &dyn Controller) {
    async fn checks(
        metadata: AppMetadata,
        test_init_key: String,
        test_init_value: String,
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
    ) -> Result<()> {
        assert_http_response(
            get_url(
                metadata.base.as_str(),
                &format!("/test?testkey={test_init_key}&testval={test_init_value}"),
            )
            .as_str(),
            Method::GET,
            "",
            200,
            &[],
            None,
        )
        .await
    }

    let init_key = uuid::Uuid::new_v4().to_string();
    let init_value = uuid::Uuid::new_v4().to_string();

    let tc = TestCaseBuilder::default()
        .name("sqlite".to_string())
        .appname(Some("sqlite".to_string()))
        .template(None)
        .deploy_args(vec![
            "--sqlite".to_string(),
            "\"CREATE TABLE testdata(key TEXT, value TEXT)\"".to_string(),
            "--sqlite".to_string(),
            format!("\"INSERT INTO testdata(key, value) VALUES ('{init_key}', '{init_value}')\""),
        ])
        .assertions(
            move |metadata: AppMetadata,
                  stdout_stream: Option<Pin<Box<dyn AsyncBufRead>>>,
                  stderr_stream: Option<Pin<Box<dyn AsyncBufRead>>>| {
                let ik = init_key.clone();
                let iv = init_value.clone();
                Box::pin(checks(metadata, ik, iv, stdout_stream, stderr_stream))
            },
        )
        .build()
        .unwrap();

    tc.run(controller).await.unwrap()
}

pub async fn sqlite_validation_works(controller: &dyn Controller) {
    async fn checks(
        _: AppMetadata,
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
        stderr: Option<Pin<Box<dyn AsyncBufRead>>>,
    ) -> Result<()> {
        let err_text = utils::get_output(stderr).await.unwrap();
        let expected1 = "Component hello uses database 'anaspeptic'";
        let expected2 = "Component hello uses database 'pericombobulations'";

        assert!(
            err_text.contains(expected1),
            "Expected error containing '{expected1}' but got '{err_text}'"
        );
        assert!(
            err_text.contains(expected2),
            "Expected error containing '{expected2}' but got '{err_text}'"
        );
        Ok(())
    }

    let tc = TestCaseBuilder::default()
        .name("sqlite-undefined-db".to_string())
        .appname(Some("sqlite-undefined-db".to_string()))
        .template(None)
        .assertions(
            |metadata: AppMetadata,
             stdout_stream: Option<Pin<Box<dyn AsyncBufRead>>>,
             stderr_stream: Option<Pin<Box<dyn AsyncBufRead>>>| {
                Box::pin(checks(metadata, stdout_stream, stderr_stream))
            },
        )
        .build()
        .unwrap();

    tc.try_run(controller).await.unwrap();
}

pub async fn http_python_works(controller: &dyn Controller) {
    async fn checks(
        metadata: AppMetadata,
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
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
             stdout_stream: Option<Pin<Box<dyn AsyncBufRead>>>,
             stderr_stream: Option<Pin<Box<dyn AsyncBufRead>>>| {
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
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
    ) -> Result<()> {
        assert_http_response(
            get_url(metadata.base.as_str(), "/index.php").as_str(),
            Method::GET,
            "",
            200,
            &[],
            Some("Hello Fermyon Spin"),
        )
        .await
    }

    let tc = TestCaseBuilder::default()
        .name("http-php-template".to_string())
        .template(Some("http-php".to_string()))
        .assertions(
            |metadata: AppMetadata,
             stdout_stream: Option<Pin<Box<dyn AsyncBufRead>>>,
             stderr_stream: Option<Pin<Box<dyn AsyncBufRead>>>| {
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
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
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
             stdout_stream: Option<Pin<Box<dyn AsyncBufRead>>>,
             stderr_stream: Option<Pin<Box<dyn AsyncBufRead>>>| {
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
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
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
             stdout_stream: Option<Pin<Box<dyn AsyncBufRead>>>,
             stderr_stream: Option<Pin<Box<dyn AsyncBufRead>>>| {
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
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
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
             stdout_stream: Option<Pin<Box<dyn AsyncBufRead>>>,
             stderr_stream: Option<Pin<Box<dyn AsyncBufRead>>>| {
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
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
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
             stdout_stream: Option<Pin<Box<dyn AsyncBufRead>>>,
             stderr_stream: Option<Pin<Box<dyn AsyncBufRead>>>| {
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
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
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
             stdout_stream: Option<Pin<Box<dyn AsyncBufRead>>>,
             stderr_stream: Option<Pin<Box<dyn AsyncBufRead>>>| {
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
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
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
             stdout_stream: Option<Pin<Box<dyn AsyncBufRead>>>,
             stderr_stream: Option<Pin<Box<dyn AsyncBufRead>>>| {
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
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
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
             stdout_stream: Option<Pin<Box<dyn AsyncBufRead>>>,
             stderr_stream: Option<Pin<Box<dyn AsyncBufRead>>>| {
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
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
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
             stdout_stream: Option<Pin<Box<dyn AsyncBufRead>>>,
             stderr_stream: Option<Pin<Box<dyn AsyncBufRead>>>| {
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
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
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
             stdout_stream: Option<Pin<Box<dyn AsyncBufRead>>>,
             stderr_stream: Option<Pin<Box<dyn AsyncBufRead>>>| {
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
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
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
             stdout_stream: Option<Pin<Box<dyn AsyncBufRead>>>,
             stderr_stream: Option<Pin<Box<dyn AsyncBufRead>>>| {
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
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
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
             stdout_stream: Option<Pin<Box<dyn AsyncBufRead>>>,
             stderr_stream: Option<Pin<Box<dyn AsyncBufRead>>>| {
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
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
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
             stdout_stream: Option<Pin<Box<dyn AsyncBufRead>>>,
             stderr_stream: Option<Pin<Box<dyn AsyncBufRead>>>| {
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
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
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
             stdout_stream: Option<Pin<Box<dyn AsyncBufRead>>>,
             stderr_stream: Option<Pin<Box<dyn AsyncBufRead>>>| {
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
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
        stderr_stream: Option<Pin<Box<dyn AsyncBufRead>>>,
    ) -> Result<()> {
        //TODO: wait for spin up to be ready dynamically
        sleep(Duration::from_secs(10)).await;

        let output = utils::run(
            &[
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
        utils::assert_success(&output);

        let stderr = utils::get_output_stream(stderr_stream, Duration::from_secs(5)).await?;
        let expected_logs = vec!["Payload::::", "msg-from-go-channel"];

        assert!(expected_logs
            .iter()
            .all(|item| stderr.contains(&item.to_string())),
        "Expected log lines to contain all of {expected_logs:?} but actual lines were '{stderr:?}'");

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
             stdout_stream: Option<Pin<Box<dyn AsyncBufRead>>>,
             stderr_stream: Option<Pin<Box<dyn AsyncBufRead>>>| {
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
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
        stderr_stream: Option<Pin<Box<dyn AsyncBufRead>>>,
    ) -> Result<()> {
        //TODO: wait for spin up to be ready dynamically
        sleep(Duration::from_secs(20)).await;

        utils::run(
            &[
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

        let stderr = utils::get_output_stream(stderr_stream, Duration::from_secs(5)).await?;

        let expected_logs = vec!["msg-from-rust-channel"];

        assert!(expected_logs
            .iter()
            .all(|item| stderr.contains(&item.to_string())),
        "Expected log lines to contain all of {expected_logs:?} but actual lines were '{stderr:?}'");

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
             stdout_stream: Option<Pin<Box<dyn AsyncBufRead>>>,
             stderr_stream: Option<Pin<Box<dyn AsyncBufRead>>>| {
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
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
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
             stdout_stream: Option<Pin<Box<dyn AsyncBufRead>>>,
             stderr_stream: Option<Pin<Box<dyn AsyncBufRead>>>| {
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
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
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
             stdout_stream: Option<Pin<Box<dyn AsyncBufRead>>>,
             stderr_stream: Option<Pin<Box<dyn AsyncBufRead>>>| {
                Box::pin(checks(metadata, stdout_stream, stderr_stream))
            },
        )
        .build()
        .unwrap();

    tc.run(controller).await.unwrap()
}

pub async fn longevity_apps_works(controller: &dyn Controller) {
    let current_spin_version = spin::version().unwrap_or_else(|err| {
        println!("error getting spin version {}", err);
        String::new()
    });

    // version of spin that was used to generate the wasm files used in this testcase
    const SPIN_VERSION_USED_TO_BUILD_APP: &str = "spin version 0.9.0 (a99ed51 2023-02-16)";

    async fn checks(
        metadata: AppMetadata,
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
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
             stdout_stream: Option<Pin<Box<dyn AsyncBufRead>>>,
             stderr_stream: Option<Pin<Box<dyn AsyncBufRead>>>| {
                Box::pin(checks(metadata, stdout_stream, stderr_stream))
            },
        )
        .build()
        .unwrap();

    if let Err(e) = tc.run(controller).await {
        panic!(
            ":\n\napp using a wasm module built by '{}' no longer works with {}\nError: {}",
            SPIN_VERSION_USED_TO_BUILD_APP, current_spin_version, e
        )
    }
}

pub async fn error_messages(controller: &dyn Controller) {
    async fn checks(
        _metadata: AppMetadata,
        _stdout: Option<Pin<Box<dyn AsyncBufRead>>>,
        stderr: Option<Pin<Box<dyn AsyncBufRead>>>,
    ) -> Result<()> {
        let appdir = spin::appdir("error");
        let expected = tokio::fs::read_to_string(appdir.join("error.txt"))
            .await
            .unwrap()
            .replace(
                "$APPDIR",
                &appdir.canonicalize().unwrap().display().to_string(),
            );
        let actual = utils::get_output(stderr).await.unwrap();
        assert_eq!(actual, expected);
        Ok(())
    }

    let tc = TestCaseBuilder::default()
        .name("error".to_string())
        .appname(Some("error".to_string()))
        .assertions(
            |metadata: AppMetadata,
             stdout_stream: Option<Pin<Box<dyn AsyncBufRead>>>,
             stderr_stream: Option<Pin<Box<dyn AsyncBufRead>>>| {
                Box::pin(checks(metadata, stdout_stream, stderr_stream))
            },
        )
        .build()
        .unwrap();

    tc.try_run(controller).await.unwrap();
}
