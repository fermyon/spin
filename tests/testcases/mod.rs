use anyhow::Result;
use e2e_testing::controller::Controller;
use e2e_testing::http_asserts::assert_http_response;
use e2e_testing::metadata::AppMetadata;
use e2e_testing::testcase::TestCaseBuilder;
use e2e_testing::{spin, utils};
use hyper::Method;
use std::pin::Pin;
use std::time::Duration;
use tokio::io::AsyncBufRead;
use tokio::time::sleep;

pub async fn redis_go_works(controller: &dyn Controller) {
    async fn checks(
        _: AppMetadata,
        _: Option<Pin<Box<dyn AsyncBufRead>>>,
        stderr_stream: Option<Pin<Box<dyn AsyncBufRead>>>,
    ) -> Result<()> {
        wait_for_spin().await;

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

        let stderr = get_output_stream(stderr_stream).await?;
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
        .pre_build_hooks(Some(vec![vec![
            "go".to_string(),
            "mod".to_string(),
            "tidy".to_string(),
        ]]))
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
        wait_for_spin().await;

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

        let stderr = get_output_stream(stderr_stream).await?;

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
        .pre_build_hooks(Some(vec![vec![
            "go".to_string(),
            "mod".to_string(),
            "tidy".to_string(),
        ]]))
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

async fn get_output_stream(
    stream: Option<Pin<Box<dyn AsyncBufRead>>>,
) -> anyhow::Result<Vec<String>> {
    utils::get_output_stream(stream, Duration::from_secs(5)).await
}

async fn wait_for_spin() {
    //TODO: wait for spin up to be ready dynamically
    sleep(Duration::from_secs(10)).await;
}
