use anyhow::Result;
use hyper::client::HttpConnector;
use hyper::{body, Body, Client, Request, Response};
use hyper_tls::HttpsConnector;
use std::str;

pub async fn assert_status(url: &str, expected: u16) -> Result<()> {
    let resp = make_request("GET", url, "").await?;
    let status = resp.status();

    let response = body::to_bytes(resp.into_body()).await.unwrap().to_vec();
    let actual_body = str::from_utf8(&response).unwrap().to_string();

    assert_eq!(status, expected, "{}", actual_body);

    Ok(())
}

pub async fn assert_http_response(
    url: &str,
    expected: u16,
    expected_headers: &[(&str, &str)],
    expected_body: Option<&str>,
) -> Result<()> {
    let res = make_request("GET", url, "").await?;

    let status = res.status();
    assert_eq!(expected, status.as_u16());

    let headers = res.headers();
    for (k, v) in expected_headers {
        assert_eq!(
            &headers
                .get(k.to_string())
                .unwrap_or_else(|| panic!("cannot find header {}", k))
                .to_str()?,
            v
        )
    }

    if let Some(expected_body_str) = expected_body {
        let response = body::to_bytes(res.into_body()).await.unwrap().to_vec();
        let actual_body = str::from_utf8(&response).unwrap().to_string();
        assert_eq!(expected_body_str, actual_body);
    }

    Ok(())
}

pub async fn create_request(method: &str, url: &str, body: &str) -> Result<Request<Body>> {
    let req = Request::builder()
        .method(method)
        .uri(url)
        .body(Body::from(body.to_string()))
        .expect("request builder");

    Ok(req)
}

pub fn create_client() -> Client<HttpsConnector<HttpConnector>> {
    let connector = HttpsConnector::new();
    Client::builder().build::<_, hyper::Body>(connector)
}

pub async fn make_request(method: &str, path: &str, body: &str) -> Result<Response<Body>> {
    let c = create_client();
    let req = create_request(method, path, body);

    let resp = c.request(req.await?).await.unwrap();
    Ok(resp)
}
