use anyhow::Result;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use url::Url;

const DEFAULT_BRANCH: &str = "main";

/// Enables cloning and fetching the latest of a git repository to a local
/// directory.
pub struct GitSource {
    /// Address to remote git repository.
    source_url: Url,
    /// Branch to clone/fetch.
    branch: String,
    /// Destination to clone repository into.
    git_root: PathBuf,
}

impl GitSource {
    /// Creates a new git source
    pub fn new(source_url: &Url, branch: Option<String>, git_root: impl AsRef<Path>) -> GitSource {
        Self {
            source_url: source_url.clone(),
            branch: branch.unwrap_or_else(|| DEFAULT_BRANCH.to_owned()),
            git_root: git_root.as_ref().to_owned(),
        }
    }

    /// Clones a contents of a git repository to a local directory
    pub async fn clone_repo(&self) -> Result<()> {
        let mut git = Command::new("git");
        git.args([
            "clone",
            self.source_url.as_ref(),
            "--branch",
            &self.branch,
            "--single-branch",
        ])
        .arg(&self.git_root);
        let clone_result = git.output().await?;
        if !clone_result.status.success() {
            anyhow::bail!(
                "Error cloning Git repo {}: {}",
                self.source_url,
                String::from_utf8_lossy(&clone_result.stderr)
            )
        }
        Ok(())
    }

    /// Fetches the latest changes from the source repository
    pub async fn pull(&self) -> Result<()> {
        let mut git = Command::new("git");
        git.arg("-C").arg(&self.git_root).arg("pull");
        let pull_result = git.output().await?;
        if !pull_result.status.success() {
            anyhow::bail!(
                "Error updating Git repo at {}: {}",
                self.git_root.display(),
                String::from_utf8_lossy(&pull_result.stderr)
            )
        }
        Ok(())
    }
}
