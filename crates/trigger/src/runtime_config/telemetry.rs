use serde::Deserialize;
use url::Url;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TelemetryOpts {
    Otlp(OtlpOpts),
}

#[derive(Debug, Deserialize)]
pub struct OtlpOpts {
    pub endpoint: Url,
    pub traces: bool,
    pub metrics: bool,
}
