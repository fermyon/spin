//! Package for working with Wasm component templates.

#![deny(missing_docs)]

use anyhow::{bail, Context, Result};
use fs_extra::dir::CopyOptions;
use git2::{build::RepoBuilder, Repository};
use std::path::{Path, PathBuf};
use tokio::fs;
use walkdir::WalkDir;

const SPIN_DIR: &str = "spin";
const TEMPLATES_DIR: &str = "templates";
const LOCAL_TEMPLATES: &str = "local";

/// A WebAssembly component template repository
#[derive(Clone, Debug, Default)]
pub struct TemplateRepository {
    /// The name of the template repository
    pub name: String,
    /// The git repository
    pub git: Option<String>,
    /// The branch of the git repository.
    pub branch: Option<String>,
    /// List of templates in the repository.
    pub templates: Vec<String>,
}

/// A templates manager that handles the local cache.
pub struct TemplatesManager {
    root: PathBuf,
}

impl TemplatesManager {
    /// Creates a cache using the default root directory.
    pub async fn default() -> Result<Self> {
        let mut root = dirs::cache_dir().context("cannot get system cache directory")?;
        root.push(SPIN_DIR);

        Ok(Self::new(root)
            .await
            .context("failed to create cache root directory")?)
    }

    /// Creates a cache using the given root directory.
    pub async fn new(dir: impl Into<PathBuf>) -> Result<Self> {
        let root = dir.into();

        Self::ensure(&root).await?;
        Self::ensure(&root.join(TEMPLATES_DIR)).await?;
        Self::ensure(&root.join(TEMPLATES_DIR).join(LOCAL_TEMPLATES)).await?;
        Self::ensure(
            &root
                .join(TEMPLATES_DIR)
                .join(LOCAL_TEMPLATES)
                .join(TEMPLATES_DIR),
        )
        .await?;

        let cache = Self { root };
        Ok(cache)
    }

    /// Adds the given templates repository locally and offline by cloning it.
    pub fn add_repo(&self, name: &str, url: &str, branch: Option<&str>) -> Result<()> {
        let dst = &self.root.join(TEMPLATES_DIR).join(name);
        log::debug!("adding repository {} to {:?}", url, dst);

        match branch {
            Some(b) => RepoBuilder::new().branch(b).clone(url, dst)?,
            None => RepoBuilder::new().clone(url, dst)?,
        };

        Ok(())
    }

    /// Add a local directory as a template.
    pub fn add_local(&self, name: &str, src: &Path) -> Result<()> {
        let src = std::fs::canonicalize(src)?;
        let dst = &self
            .root
            .join(TEMPLATES_DIR)
            .join(LOCAL_TEMPLATES)
            .join(TEMPLATES_DIR)
            .join(name);
        log::debug!("adding local template from {:?} to {:?}", src, dst);

        symlink::symlink_dir(src, dst)?;
        Ok(())
    }

    /// Generate a new project given a template name from a template repository.
    pub async fn generate(&self, repo: &str, template: &str, dst: PathBuf) -> Result<()> {
        let src = self.get_path(repo, template)?;
        let mut opts = CopyOptions::new();
        opts.copy_inside = true;
        let _ = fs_extra::dir::copy(src, dst, &opts)?;
        Ok(())
    }

    /// Lists all the templates repositories.
    pub async fn list(&self) -> Result<Vec<TemplateRepository>> {
        let mut res = vec![];
        let templates = &self.root.join(TEMPLATES_DIR);

        // Search the top-level directories in $XDG_CACHE/spin/templates.
        for tr in WalkDir::new(templates).max_depth(1).follow_links(true) {
            let tr = tr?.clone();
            if tr.path().eq(templates) || !tr.path().is_dir() {
                continue;
            }
            let name = Self::path_to_name(tr.clone().path());
            let mut templates = vec![];
            let td = tr.clone().path().join(TEMPLATES_DIR);
            for t in WalkDir::new(td.clone()).max_depth(1).follow_links(true) {
                let t = t?.clone();
                if t.path().eq(&td) || !t.path().is_dir() {
                    continue;
                }
                templates.push(Self::path_to_name(t.path()));
            }

            let repo = match Repository::open(tr.clone().path()) {
                Ok(repo) => TemplateRepository {
                    name,
                    git: repo
                        .find_remote(repo.remotes()?.get(0).unwrap_or("origin"))?
                        .url()
                        .map(|s| s.to_string()),
                    branch: repo.head().unwrap().name().map(|s| s.to_string()),
                    templates,
                },
                Err(_) => TemplateRepository {
                    name,
                    git: None,
                    branch: None,
                    templates,
                },
            };
            res.push(repo);
        }

        Ok(res)
    }

    /// Get the path of a template from the given repository.
    fn get_path(&self, repo: &str, template: &str) -> Result<PathBuf> {
        let repo_path = &self.root.join(TEMPLATES_DIR).join(repo);
        if !repo_path.exists() {
            bail!("cannot find templates repository {} locally", repo)
        }

        let template_path = repo_path.join(TEMPLATES_DIR).join(template);
        if !template_path.exists() {
            bail!("cannot find template {} in repository {}", template, repo);
        }

        Ok(template_path)
    }

    /// Ensure the root directory exists, or else create it.
    async fn ensure(root: &Path) -> Result<()> {
        if !root.exists() {
            log::debug!("creating cache root directory `{}`", root.display());
            fs::create_dir_all(root).await.with_context(|| {
                format!("failed to create cache root directory `{}`", root.display())
            })?;
        } else if !root.is_dir() {
            bail!(
                "cache root `{}` already exists and is not a directory",
                root.display()
            );
        } else {
            log::debug!("using existing cache root directory `{}`", root.display());
        }

        Ok(())
    }

    fn path_to_name(p: &Path) -> String {
        p.file_name().unwrap().to_str().unwrap().to_string()
    }
}
