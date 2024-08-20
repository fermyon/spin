//! The Spin runtime running in the same process as the test

use std::{path::PathBuf, sync::Arc};

use anyhow::Context as _;
use spin_http::routes::RouteMatch;
use spin_trigger2::cli::{TriggerAppBuilder, TriggerAppOptions};
use spin_trigger_http2::{HttpServer, HttpTrigger};
use test_environment::{
    http::{Request, Response},
    services::ServicesConfig,
    Runtime, TestEnvironment, TestEnvironmentConfig,
};

/// An instance of Spin running in the same process as the tests instead of as a separate process
///
/// Use `runtimes::spin_cli::SpinCli` if you'd rather use Spin as a separate process
pub struct InProcessSpin {
    server: Arc<HttpServer>,
}

impl InProcessSpin {
    /// Configure a new instance of Spin running in the same process as the tests
    pub fn config(
        services_config: ServicesConfig,
        preboot: impl FnOnce(&mut TestEnvironment<InProcessSpin>) -> anyhow::Result<()> + 'static,
    ) -> TestEnvironmentConfig<Self> {
        TestEnvironmentConfig {
            services_config,
            create_runtime: Box::new(|env| {
                preboot(env)?;
                tokio::runtime::Runtime::new()
                    .context("failed to start tokio runtime")?
                    .block_on(async { initialize_trigger(env).await })
            }),
        }
    }

    /// Create a new instance of Spin running in the same process as the tests
    pub fn new(server: Arc<HttpServer>) -> Self {
        Self { server }
    }

    /// Make an HTTP request to the Spin instance
    pub fn make_http_request(&self, req: Request<'_, &[u8]>) -> anyhow::Result<Response> {
        tokio::runtime::Runtime::new()?.block_on(async {
            let method: reqwest::Method = req.method.into();
            let mut builder = http::request::Request::builder()
                .method(method)
                .uri(req.path);

            for (key, value) in req.headers {
                builder = builder.header(*key, *value);
            }
            // TODO(rylev): convert body as well
            let req = builder.body(spin_http::body::empty()).unwrap();
            let route_match = RouteMatch::synthetic("test", "/");
            let response = self
                .server
                .handle_trigger_route(
                    req,
                    route_match,
                    http::uri::Scheme::HTTP,
                    (std::net::Ipv4Addr::LOCALHOST, 7000).into(),
                )
                .await?;
            use http_body_util::BodyExt;
            let status = response.status().as_u16();
            let headers = response
                .headers()
                .iter()
                .map(|(k, v)| {
                    (
                        k.as_str().to_owned(),
                        String::from_utf8(v.as_bytes().to_owned()).unwrap(),
                    )
                })
                .collect();
            let body = response.into_body();
            let chunks = body
                .collect()
                .await
                .context("could not get runtime test HTTP response")?
                .to_bytes()
                .to_vec();
            Ok(Response::full(status, headers, chunks))
        })
    }
}

impl Runtime for InProcessSpin {
    fn error(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}

/// Initialize the trigger for the Spin instance inside the environment
async fn initialize_trigger(
    env: &mut TestEnvironment<InProcessSpin>,
) -> anyhow::Result<InProcessSpin> {
    let locked_app = spin_loader::from_file(
        env.path().join("spin.toml"),
        spin_loader::FilesMountStrategy::Direct,
        None,
    )
    .await?;

    let app = spin_app::App::new("my-app", locked_app);
    let trigger = HttpTrigger::new(&app, "127.0.0.1:80".parse().unwrap(), None)?;
    let mut builder = TriggerAppBuilder::new(trigger, PathBuf::from("."));
    let trigger_app = builder.build(app, TriggerAppOptions::default()).await?;
    let server = builder.trigger.into_server(trigger_app)?;

    Ok(InProcessSpin::new(server))
}
