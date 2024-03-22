use anyhow::Result;
use std::io::ErrorKind;
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
        let clone_result = git.output().await.understand_git_result();
        if let Err(e) = clone_result {
            anyhow::bail!("Error cloning Git repo {}: {}", self.source_url, e)
        }
        Ok(())
    }

    /// Fetches the latest changes from the source repository
    pub async fn pull(&self) -> Result<()> {
        let mut git = Command::new("git");
        git.arg("-C").arg(&self.git_root).arg("pull");
        let pull_result = git.output().await.understand_git_result();
        if let Err(e) = pull_result {
            anyhow::bail!(
                "Error updating Git repo at {}: {}",
                self.git_root.display(),
                e
            )
        }
        Ok(())
    }
}

// TODO: the following and templates/git.rs are duplicates

pub(crate) enum GitError {
    ProgramFailed(Vec<u8>),
    ProgramNotFound,
    Other(anyhow::Error),
}

impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProgramNotFound => f.write_str("`git` command not found - is git installed?"),
            Self::Other(e) => e.fmt(f),
            Self::ProgramFailed(stderr) => match std::str::from_utf8(stderr) {
                Ok(s) => f.write_str(s),
                Err(_) => f.write_str("(cannot get error)"),
            },
        }
    }
}

pub(crate) trait UnderstandGitResult {
    fn understand_git_result(self) -> Result<Vec<u8>, GitError>;
}

impl UnderstandGitResult for Result<std::process::Output, std::io::Error> {
    fn understand_git_result(self) -> Result<Vec<u8>, GitError> {
        match self {
            Ok(output) => {
                if output.status.success() {
                    Ok(output.stdout)
                } else {
                    Err(GitError::ProgramFailed(output.stderr))
                }
            }
            Err(e) => match e.kind() {
                // TODO: consider cases like insufficient permission?
                ErrorKind::NotFound => Err(GitError::ProgramNotFound),
                _ => {
                    let err = anyhow::Error::from(e).context("Failed to run `git` command");
                    Err(GitError::Other(err))
                }
            },
        }
    }
}
