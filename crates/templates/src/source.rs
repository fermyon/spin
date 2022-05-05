use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context};
use tempfile::{tempdir, TempDir};
use tokio::process::Command;
use url::Url;

use crate::directory::subdirectories;

const TEMPLATE_SOURCE_DIR: &str = "templates";

#[derive(Debug)]
pub enum TemplateSource {
    Git { url: Url, branch: Option<String> },
    File(PathBuf),
}

impl TemplateSource {
    pub fn try_from_git(git_url: impl AsRef<str>, branch: &Option<String>) -> anyhow::Result<Self> {
        let url_str = git_url.as_ref();
        let url =
            Url::parse(url_str).with_context(|| format!("Failed to parse {} as URL", url_str))?;
        Ok(Self::Git {
            url,
            branch: branch.clone(),
        })
    }
}

pub struct LocalTemplateSource {
    root: PathBuf,
    _temp_dir: Option<TempDir>,
}

impl TemplateSource {
    pub async fn get_local(&self) -> anyhow::Result<LocalTemplateSource> {
        match self {
            Self::Git { url, branch } => clone_local(url, branch).await,
            Self::File(path) => check_local(path).await,
        }
    }

    pub fn requires_copy(&self) -> bool {
        match self {
            Self::Git { .. } => true,
            Self::File(_) => false,
        }
    }
}

impl LocalTemplateSource {
    pub async fn template_directories(&self) -> anyhow::Result<Vec<PathBuf>> {
        let templates_root = self.root.join(TEMPLATE_SOURCE_DIR);
        if templates_root.exists() {
            subdirectories(&templates_root).with_context(|| {
                format!(
                    "Failed to read contents of '{}' directory",
                    TEMPLATE_SOURCE_DIR
                )
            })
        } else {
            Err(anyhow!(
                "Template source {} does not contain a '{}' directory",
                self.root.display(),
                TEMPLATE_SOURCE_DIR
            ))
        }
    }
}

async fn clone_local(url: &Url, branch: &Option<String>) -> anyhow::Result<LocalTemplateSource> {
    let temp_dir = tempdir()?;
    let path = temp_dir.path().to_owned();

    let url_str = url.as_str();

    let mut git = Command::new("git");
    git.arg("clone");

    if let Some(b) = branch {
        git.arg("--branch").arg(b);
    }

    let clone_result = git.arg(&url_str).arg(&path).output().await?;
    match clone_result.status.success() {
        true => Ok(LocalTemplateSource {
            root: path,
            _temp_dir: Some(temp_dir),
        }),
        false => Err(anyhow!(
            "Error cloning Git repo {}: {}",
            url_str,
            String::from_utf8(clone_result.stderr)
                .unwrap_or_else(|_| "(cannot get error)".to_owned())
        )),
    }
}

async fn check_local(path: &Path) -> anyhow::Result<LocalTemplateSource> {
    if path.exists() {
        Ok(LocalTemplateSource {
            root: path.to_owned(),
            _temp_dir: None,
        })
    } else {
        Err(anyhow!("Path not found: {}", path.display()))
    }
}
