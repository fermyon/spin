use cloud_openapi::models::TokenInfo;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ConnectionConfig {
    pub insecure: bool,
    pub token: TokenInfo,
    pub url: String,
}
