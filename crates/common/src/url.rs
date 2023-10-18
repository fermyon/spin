//! Operations on URLs

use anyhow::{anyhow, Context};

use std::path::PathBuf;

/// Parse the path from a 'file:' URL
pub fn parse_file_url(url: &str) -> anyhow::Result<PathBuf> {
    url::Url::parse(url)
        .with_context(|| format!("Invalid URL: {url:?}"))?
        .to_file_path()
        .map_err(|_| anyhow!("Invalid file URL path: {url:?}"))
}
