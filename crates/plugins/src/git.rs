use anyhow::{anyhow, Result};
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
    pub fn new(
        source_url: &Url,
        branch: Option<String>,
        git_root: impl AsRef<Path>,
    ) -> Result<GitSource> {
        Ok(Self {
            source_url: source_url.clone(),
            branch: branch.unwrap_or_else(|| DEFAULT_BRANCH.to_owned()),
            git_root: git_root.as_ref().to_owned(),
        })
    }

    /// Clones a contents of a git repository to a local directory
    pub async fn clone(&self) -> Result<()> {
        let mut git = Command::new("git");
        git.args([
            "clone",
            self.source_url.as_ref(),
            "--branch",
            &self.branch,
            "--single-branch",
            &self.git_root.to_string_lossy(),
        ]);
        let clone_result = git.output().await?;
        match clone_result.status.success() {
            true => Ok(()),
            false => Err(anyhow!(
                "Error cloning Git repo {}: {}",
                self.source_url,
                String::from_utf8(clone_result.stderr)
                    .unwrap_or_else(|_| "(cannot get error)".to_owned())
            )),
        }
    }

    /// Fetches the latest changes from the source repository
    pub async fn pull(&self) -> Result<()> {
        let mut git = Command::new("git");
        git.args(["-C", &self.git_root.to_string_lossy(), "pull"]);
        let pull_result = git.output().await?;
        match pull_result.status.success() {
            true => Ok(()),
            false => Err(anyhow!(
                "Error updating Git repo at {}: {}",
                self.git_root.display(),
                String::from_utf8(pull_result.stderr)
                    .unwrap_or_else(|_| "(cannot update error)".to_owned())
            )),
        }
    }
}
