use {
    anyhow::{anyhow, Result},
    spin_sdk::{
        http::{self, IncomingResponse, IntoResponse, Method, Request, RequestBuilder, Response},
        http_component,
    },
};

#[http_component]
async fn handle(req: Request) -> Result<impl IntoResponse> {
    let request = RequestBuilder::new(Method::Post, "/")
        .uri(
            req.header("url")
                .ok_or_else(|| anyhow!("missing url header"))?
                .as_str()
                .ok_or_else(|| anyhow!("invalid utf-8 in url header value"))?,
        )
        .method(Method::Post)
        .body("Hello, world!")
        .build();

    let response: IncomingResponse = http::send(request).await?;
    let status = response.status();

    Ok(Response::builder()
        .status(200)
        .header("content-type", "text/plain")
        .body(format!("response status: {status}"))
        .build())
}
