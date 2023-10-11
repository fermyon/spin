use {
    self::wasi::http::types::{
        Fields, IncomingRequest, Method, OutgoingBody, OutgoingRequest, OutgoingResponse,
        ResponseOutparam, Scheme,
    },
    anyhow::{bail, Result},
    futures::{stream, FutureExt, SinkExt, StreamExt, TryStreamExt},
    sha2::{Digest, Sha256},
    spin_sdk::wasi_http_component,
    std::str,
    url::Url,
};

const MAX_CONCURRENCY: usize = 16;

#[wasi_http_component]
async fn handle_request(request: IncomingRequest, response_out: ResponseOutparam) -> Result<()> {
    let method = request.method();
    let path = request.path_with_query();
    let headers = request.headers().entries();

    match (method, path.as_deref()) {
        (Method::Get, Some("/hash-all")) => {
            let urls = headers.iter().filter_map(|(k, v)| {
                (k == "url")
                    .then_some(v)
                    .and_then(|v| str::from_utf8(v).ok())
                    .and_then(|v| Url::parse(v).ok())
            });

            let results = urls.map(move |url| hash(url.clone()).map(move |result| (url, result)));

            let mut results = stream::iter(results).buffer_unordered(MAX_CONCURRENCY);

            let response = OutgoingResponse::new(
                200,
                &Fields::new(&[("content-type".to_string(), b"text/plain".to_vec())]),
            );

            let mut sink = executor::outgoing_response_body(&response);

            ResponseOutparam::set(response_out, Ok(response));

            while let Some((url, result)) = results.next().await {
                sink.send(
                    match result {
                        Ok(hash) => format!("{url}: {hash}\n"),
                        Err(e) => format!("{url}: {e:?}\n"),
                    }
                    .into_bytes(),
                )
                .await?;
            }
        }

        (Method::Post, Some("/echo")) => {
            let response = OutgoingResponse::new(
                200,
                &Fields::new(
                    &headers
                        .iter()
                        .filter_map(|(k, v)| {
                            (k == "content-type").then_some((k.clone(), v.clone()))
                        })
                        .collect::<Vec<_>>(),
                ),
            );

            let mut sink = executor::outgoing_response_body(&response);

            ResponseOutparam::set(response_out, Ok(response));

            let mut stream = executor::incoming_request_body(request);
            while let Some(chunk) = stream.try_next().await? {
                sink.send(chunk).await?;
            }
        }

        _ => {
            let response = OutgoingResponse::new(405, &Fields::new(&[]));

            let body = response.write().expect("response should be writable");

            ResponseOutparam::set(response_out, Ok(response));

            OutgoingBody::finish(body, None);
        }
    }

    Ok(())
}

async fn hash(url: Url) -> Result<String> {
    let request = OutgoingRequest::new(
        &Method::Get,
        Some(url.path()),
        Some(&match url.scheme() {
            "http" => Scheme::Http,
            "https" => Scheme::Https,
            scheme => Scheme::Other(scheme.into()),
        }),
        Some(url.authority()),
        &Fields::new(&[]),
    );

    let response = executor::outgoing_request_send(request).await?;

    let status = response.status();

    if !(200..300).contains(&status) {
        bail!("unexpected status: {status}");
    }

    let mut body = executor::incoming_response_body(response);

    let mut hasher = Sha256::new();
    while let Some(chunk) = body.try_next().await? {
        hasher.update(&chunk);
    }

    Ok(hex::encode(hasher.finalize()))
}
