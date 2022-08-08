#![deny(missing_docs)]

use anyhow::{Context, Result};
use reqwest::Url;

// Check whether http host can be parsed by Url
pub fn validate_allowed_http_hosts(http_hosts: &Option<Vec<String>>) -> Result<()> {
    if let Some(domains) = http_hosts.as_deref() {
        if domains
            .iter()
            .any(|domain| domain == wasi_outbound_http::ALLOW_ALL_HOSTS)
        {
            return Ok(());
        }
        let _ = domains
            .iter()
            .map(|d| {
                Url::parse(d).with_context(|| format!("Can't parse {} in allowed_http_hosts", d))
            })
            .collect::<Result<Vec<_>, _>>()?;
    }
    Ok(())
}
