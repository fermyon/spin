use anyhow::{anyhow, Result};
use std::path::PathBuf;
use tokio::process::Command;
use url::Url;

const PLUGINS_REPO_BRANCH: &str = "main";

pub struct GitSource {
    source_url: Url,
    branch: String,
    local_repo_dir: PathBuf,
}

impl GitSource {
    pub fn new(
        source_url: &Url,
        branch: Option<String>,
        local_repo_dir: PathBuf,
    ) -> Result<GitSource> {
        Ok(Self {
            source_url: source_url.clone(),
            branch: branch.unwrap_or_else(|| PLUGINS_REPO_BRANCH.to_owned()),
            local_repo_dir,
        })
    }

    pub async fn clone(&self) -> Result<()> {
        let mut git = Command::new("git");
        git.args([
            "clone",
            self.source_url.as_ref(),
            "--branch",
            &self.branch,
            "--single-branch",
            &self.local_repo_dir.to_string_lossy(),
        ]);
        let clone_result = git.output().await?;
        match clone_result.status.success() {
            true => {
                println!("Cloned Repository Successfully!");
                Ok(())
            }
            false => Err(anyhow!(
                "Error cloning Git repo {}: {}",
                self.source_url,
                String::from_utf8(clone_result.stderr)
                    .unwrap_or_else(|_| "(cannot get error)".to_owned())
            )),
        }
    }

    pub async fn pull(&self) -> Result<()> {
        let mut git = Command::new("git");
        git.args(["-C", &self.local_repo_dir.to_string_lossy(), "pull"]);
        let pull_result = git.output().await?;
        match pull_result.status.success() {
            true => {
                println!("Updated repository successfully");
                Ok(())
            }
            false => Err(anyhow!(
                "Error updating Git repo at {:?}: {}",
                self.local_repo_dir,
                String::from_utf8(pull_result.stderr)
                    .unwrap_or_else(|_| "(cannot update error)".to_owned())
            )),
        }
    }
}
