use crate::http::{Request, Response};
use crate::outbound_http::send_request;
use anyhow::Result;
use std::str::FromStr;

static ORIGIN_REQUEST_HEADER: &str = "webhook-request-origin";
static CALLBACK_REQUEST_HEADER: &str = "webhook-request-callback";
static RATE_RESPONSE_HEADER: &str = "webhook-allowed-rate";
static ORIGIN_RESPONSE_HEADER: &str = "webhook-allowed-origin";

/// validate the webhook according to this [spec](https://github.com/cloudevents/spec/blob/v1.0/http-webhook.md#4-abuse-protection).
pub fn validate_webhook(req: Request, callback_allowed: bool, rate: &str) -> Result<Response> {
    if req.method() != http::Method::OPTIONS {
        return Err(anyhow::anyhow!("invalid method"));
    }

    // check if rate is either asterisk or a positive number
    let rate = validate_rate(rate)?;

    let u = req.headers().iter().find(|h| h.0 == ORIGIN_REQUEST_HEADER);
    let origin = match u {
        Some(h) => h.1,
        None => return Err(anyhow::anyhow!("missing webhook-request-origin header")),
    };

    let mut res = http::Response::builder()
        .status(200)
        .body(Some("OK".into()))?;

    if callback_allowed {
        let callback = req
            .headers()
            .iter()
            .find(|h| h.0 == CALLBACK_REQUEST_HEADER);
        let callback = match callback {
            Some(h) => h.1,
            None => return Err(anyhow::anyhow!("missing webhook-request-callback header")),
        };
        let req = http::Request::builder()
            .method("GET")
            .header(ORIGIN_RESPONSE_HEADER, origin)
            .header(RATE_RESPONSE_HEADER, rate)
            .uri(callback.to_str()?)
            .body(None)
            .map_err(|e| anyhow::anyhow!("failed to build request: {}", e))?;
        res = send_request(req)
            .map_err(|err| anyhow::anyhow!("failed to send request with error: {:?}", err))?;
    }
    res.headers_mut()
        .insert(ORIGIN_RESPONSE_HEADER, origin.into());
    res.headers_mut()
        .insert(RATE_RESPONSE_HEADER, rate.try_into()?);
    Ok(res)
}

fn validate_rate(rate: &str) -> Result<&str> {
    if rate != "*" {
        match i32::from_str(rate) {
            Ok(rate) => {
                if rate < 1 {
                    return Err(anyhow::anyhow!(
                        "invalid rate: {}, not a positive number",
                        rate
                    ));
                }
            }
            Err(_) => {
                return Err(anyhow::anyhow!(
                    "invalid rate: {}, not a positive number or *",
                    rate
                ));
            }
        }
    };
    Ok(rate)
}

/// test validate_webhook
#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use bytes::Bytes;
    use http::{HeaderValue, Request, Response};

    #[test]
    fn test_validate_webhook() -> Result<()> {
        let req = Request::builder()
            .method(http::Method::OPTIONS)
            .header(ORIGIN_REQUEST_HEADER, "eventemitter.example.com")
            .body(Some(Bytes::from_static(b"")))
            .unwrap();

        let res = validate_webhook(req, false, "*")?;
        assert_eq!(res.status(), 200);
        assert_eq!(
            res.headers()
                .get(ORIGIN_RESPONSE_HEADER)
                .and_then(|v| Some(v.to_str().unwrap())),
            Some("eventemitter.example.com")
        );
        assert_eq!(
            res.headers()
                .get(RATE_RESPONSE_HEADER)
                .and_then(|v| Some(v.to_str().unwrap())),
            Some("*")
        );
        Ok(())
    }
}
