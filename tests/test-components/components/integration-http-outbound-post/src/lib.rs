use anyhow::Context;
use spin_sdk::{
    http::{self, IncomingResponse, IntoResponse, Method, Request, RequestBuilder, Response},
    http_component,
};

#[http_component]
async fn handle(req: Request) -> anyhow::Result<impl IntoResponse> {
    let request = RequestBuilder::new(Method::Post, "/")
        .uri(
            req.header("url")
                .context("missing url header")?
                .as_str()
                .context("invalid utf-8 in url header value")?,
        )
        .method(Method::Post)
        .header("Content-Length", "13")
        .body("Hello, world!")
        .build();

    let response: IncomingResponse = http::send(request).await?;
    let body = response
        .into_body()
        .await
        .context("failed to read response body")?;
    Ok(Response::builder()
        .status(200)
        .header("content-type", "text/plain")
        .body(body)
        .build())
}
