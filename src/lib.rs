pub mod commands;

use anyhow::Result;

const HIPPO_USERNAME_ENV: &str = "HIPPO_USERNAME";
const HIPPO_PASSWORD_ENV: &str = "HIPPO_PASSWORD";
const HIPPO_URL_ENV: &str = "HIPPO_URL";
pub struct ConnectionInfo {
    pub url: String,
    pub danger_accept_invalid_certs: bool,
    pub username: String,
    pub password: String,
}

pub fn connection_from_env() -> Result<ConnectionInfo> {
    Ok(ConnectionInfo {
        url: std::env::var(HIPPO_URL_ENV)?,
        danger_accept_invalid_certs: true,
        username: std::env::var(HIPPO_USERNAME_ENV)?,
        password: std::env::var(HIPPO_PASSWORD_ENV)?,
    })
}
