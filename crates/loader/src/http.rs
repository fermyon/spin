use std::path::Path;

use anyhow::{ensure, Context, Result};
use sha2::Digest;
use tokio::io::AsyncWriteExt;

/// Downloads content from `url` which will be verified to match `digest` and
/// then moved to `dest`.
pub async fn verified_download(url: &str, digest: &str, dest: &Path) -> Result<()> {
    tracing::debug!("Downloading content from {url:?}");

    // Prepare tempfile destination
    let prefix = format!("download-{}", digest.replace(':', "-"));
    let dest_dir = dest.parent().context("invalid dest")?;
    let (temp_file, temp_path) = tempfile::NamedTempFile::with_prefix_in(prefix, dest_dir)
        .context("error creating download tempfile")?
        .into_parts();

    // Begin download
    let mut resp = reqwest::get(url).await?.error_for_status()?;

    // Hash as we write to the tempfile
    let mut hasher = sha2::Sha256::new();
    {
        let mut temp_file = tokio::fs::File::from_std(temp_file);
        while let Some(chunk) = resp.chunk().await? {
            hasher.update(&chunk);
            temp_file.write_all(&chunk).await?;
        }
        temp_file.flush().await?;
    }

    // Check the digest
    let actual_digest = format!("sha256:{:x}", hasher.finalize());
    ensure!(
        actual_digest == digest,
        "invalid content digest; expected {digest}, downloaded {actual_digest}"
    );

    // Move to final destination
    temp_path
        .persist_noclobber(dest)
        .with_context(|| format!("Failed to save download from {url} to {}", dest.display()))?;

    Ok(())
}
