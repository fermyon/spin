use anyhow::{bail, Result};
use futures::{stream, SinkExt, StreamExt, TryStreamExt};
use spin_sdk::http::send;
use spin_sdk::http::{
    Headers, IncomingRequest, IncomingResponse, Method, OutgoingBody, OutgoingRequest,
    OutgoingResponse, ResponseOutparam, Scheme,
};
use spin_sdk::http_component;
use url::Url;

const MAX_CONCURRENCY: usize = 16;

#[http_component]
async fn handle_request(request: IncomingRequest, response_out: ResponseOutparam) {
    let headers = request.headers().entries();

    match (request.method(), request.path_with_query().as_deref()) {
        (Method::Get, Some("/hash-all")) => {
            let urls = headers.iter().filter_map(|(k, v)| {
                (k == "url")
                    .then_some(v)
                    .and_then(|v| std::str::from_utf8(v).ok())
                    .and_then(|v| Url::parse(v).ok())
            });

            let results = urls.map(|url| async move {
                let result = hash(&url).await;
                (url, result)
            });

            let mut results = stream::iter(results).buffer_unordered(MAX_CONCURRENCY);

            let response = OutgoingResponse::new(
                200,
                &Headers::new(&[("content-type".to_string(), b"text/plain".to_vec())]),
            );

            let mut body = response.take_body();

            response_out.set(response);

            while let Some((url, result)) = results.next().await {
                let payload = match result {
                    Ok(hash) => format!("{url}: {hash}\n"),
                    Err(e) => format!("{url}: {e:?}\n"),
                }
                .into_bytes();
                if let Err(e) = body.send(payload).await {
                    eprintln!("Error sending payload: {e}");
                }
            }
        }

        (Method::Post, Some("/echo")) => {
            let response = OutgoingResponse::new(
                200,
                &Headers::new(
                    &headers
                        .into_iter()
                        .filter_map(|(k, v)| (k == "content-type").then_some((k, v)))
                        .collect::<Vec<_>>(),
                ),
            );

            let mut body = response.take_body();

            response_out.set(response);

            let mut stream = request.into_body_stream();
            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(chunk) => {
                        if let Err(e) = body.send(chunk).await {
                            eprintln!("Error sending body: {e}");
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("Error receiving body: {e}");
                        break;
                    }
                }
            }
        }

        _ => {
            let response = OutgoingResponse::new(405, &Headers::new(&[]));

            let body = response.write().expect("response should be writable");

            response_out.set(response);

            OutgoingBody::finish(body, None);
        }
    }
}

async fn hash(url: &Url) -> Result<String> {
    let request = OutgoingRequest::new(
        &Method::Get,
        Some(url.path()),
        Some(&match url.scheme() {
            "http" => Scheme::Http,
            "https" => Scheme::Https,
            scheme => Scheme::Other(scheme.into()),
        }),
        Some(url.authority()),
        &Headers::new(&[]),
    );

    let response: IncomingResponse = send(request).await??;

    let status = response.status();

    if !(200..300).contains(&status) {
        bail!("unexpected status: {status}");
    }

    let mut body = response.into_body_stream();

    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    while let Some(chunk) = body.try_next().await? {
        hasher.update(&chunk);
    }

    Ok(hex::encode(hasher.finalize()))
}
