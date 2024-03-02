use url::Url;

#[derive(Debug, serde::Deserialize)]
pub struct OtlpOpts {
    _endpoint: Url,
}
