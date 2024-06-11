//! The Spin runtime running in the same process as the test

use crate::{
    http::{Request, Response},
    Runtime,
};
use anyhow::Context as _;

/// An instance of Spin running in the same process as the tests instead of as a separate process
///
/// Use `runtimes::spin_cli::SpinCli` if you'd rather use Spin as a separate process
pub struct InProcessSpin {
    trigger: spin_trigger_http::HttpTrigger,
}

impl InProcessSpin {
    pub fn new(trigger: spin_trigger_http::HttpTrigger) -> Self {
        Self { trigger }
    }

    pub fn make_http_request(&self, req: Request<'_, &[u8]>) -> anyhow::Result<Response> {
        tokio::runtime::Runtime::new()?.block_on(async {
            let method = http::Method::from_bytes(req.method.as_str().as_bytes())
                .context("could not parse runtime test HTTP method")?;
            let req = http::request::Request::builder()
                .method(method)
                .uri(req.uri)
                // TODO(rylev): convert headers and body as well
                .body(spin_http::body::empty())
                .unwrap();
            let response = self
                .trigger
                .handle(
                    req,
                    http::uri::Scheme::HTTP,
                    (std::net::Ipv4Addr::LOCALHOST, 3000).into(),
                    (std::net::Ipv4Addr::LOCALHOST, 7000).into(),
                )
                .await?;
            use http_body_util::BodyExt;
            let status = response.status().as_u16();
            let body = response.into_body();
            let chunks = body
                .collect()
                .await
                .context("could not get runtime test HTTP response")?
                .to_bytes()
                .to_vec();
            Ok(Response::full(status, Default::default(), chunks))
        })
    }
}

impl Runtime for InProcessSpin {
    fn error(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}
