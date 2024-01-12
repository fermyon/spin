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
