//! Package for working with Wasm component templates.

#![deny(missing_docs)]

mod args;
mod config;
mod files;
mod filters;
mod hooks;
mod template;
mod variable;

use anyhow::{bail, ensure, Context, Result};
pub use args::{TemplateArgs, TemplateId};
pub use config::TemplateConfig;
use console::style;
use fs_extra::dir::CopyOptions;
use git2::{build::RepoBuilder, Repository};
use indicatif::{MultiProgress, ProgressStyle};
use std::env;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use tokio::fs;
use variable::{StringEntry, Variable, VariableInfo};
use walkdir::WalkDir;

const LOCAL_TEMPLATES_REPO_PREFIX: &str = "local/templates";
const SPIN_DIR: &str = "spin";
const TEMPLATES_DIR: &str = "templates";
const LOCAL_TEMPLATES: &str = "local";
const CONFIG_FILE_NAME: &str = "spin-generate.toml";

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
    pub async fn generate(&self, args: TemplateArgs) -> Result<()> {
        let out = env::current_dir()?.join(&args.output);
        ensure!(!out.exists(), "Output directory already exists");

        let (_tmp_dir, tpl_dir, config) = self.prepare(&args)?;
        let values = args.resolve_values(None)?;

        println!(
            "{} {} {}",
            emoji::WRENCH,
            style("Generating template").bold(),
            style("...").bold()
        );

        template::expand(&tpl_dir, config, &args, values).await?;

        println!(
            "{} {} `{}`{}",
            emoji::WRENCH,
            style("Moving generated files into:").bold(),
            style(out.display()).bold().yellow(),
            style("...").bold()
        );

        fs::create_dir_all(&out).await?;
        copy_dir_all(&tpl_dir, &out)?;

        println!(
            "{} {} {}",
            emoji::SPARKLE,
            style("Done!").bold().green(),
            style("New application created").bold(),
        );

        Ok(())
    }

    /// Prepare the template
    fn prepare(&self, args: &TemplateArgs) -> Result<(TempDir, PathBuf, TemplateConfig)> {
        let src = self.get_path(args.local, &args.template_id.repo, &args.template_id.name)?;
        let tmp = tempfile::tempdir()?;
        let dst = tmp.path();

        let mut opts = CopyOptions::new();
        opts.copy_inside = true;
        fs_extra::dir::copy(&src, &dst, &opts)?;

        let tpl_dir = auto_locate_template_dir(dst, Variable::prompt)?;
        let cfg_src = tpl_dir.join(CONFIG_FILE_NAME);
        let cfg = TemplateConfig::from_path(cfg_src)?;

        Ok((tmp, tpl_dir, cfg))
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
    fn get_path(&self, local: bool, repo: &str, name: &str) -> Result<PathBuf> {
        let templates_dir = self.root.join(TEMPLATES_DIR);
        let repo_path = if local {
            templates_dir.join(LOCAL_TEMPLATES_REPO_PREFIX).join(repo)
        } else {
            templates_dir.join(repo)
        };
        if !repo_path.exists() {
            bail!("cannot find templates repository {} locally", repo)
        }

        let template_path = repo_path.join(TEMPLATES_DIR).join(&name);
        if !template_path.exists() {
            bail!("cannot find template {} in repository {}", name, repo);
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

mod emoji {
    use console::Emoji;

    pub static ERROR: Emoji<'_, '_> = Emoji("⛔  ", "");
    pub static SPARKLE: Emoji<'_, '_> = Emoji("✨  ", "");
    pub static WARN: Emoji<'_, '_> = Emoji("⚠️  ", "");
    pub static WRENCH: Emoji<'_, '_> = Emoji("🔧  ", "");
    pub static SHRUG: Emoji<'_, '_> = Emoji("🤷  ", "");
}

fn progressbar() -> MultiProgress {
    MultiProgress::new()
}

fn spinner() -> ProgressStyle {
    ProgressStyle::default_spinner()
        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
        .template("{prefix:.bold.dim} {spinner} {wide_msg}")
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<()> {
    use std::fs;

    fn check_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<()> {
        if !dst.as_ref().exists() {
            return Ok(());
        }

        for src_entry in fs::read_dir(src)? {
            let src_entry = src_entry?;
            let dst_path = dst.as_ref().join(src_entry.file_name());
            let entry_type = src_entry.file_type()?;

            if entry_type.is_dir() {
                check_dir_all(src_entry.path(), dst_path)?;
            } else if entry_type.is_file() {
                if dst_path.exists() {
                    bail!(
                        "{} {} {}",
                        emoji::WARN,
                        style("File already exists:").bold().red(),
                        style(dst_path.display()).bold().red(),
                    )
                }
            } else {
                bail!(
                    "{} {}",
                    emoji::WARN,
                    style("Symbolic links not supported").bold().red(),
                )
            }
        }
        Ok(())
    }
    fn copy_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<()> {
        fs::create_dir_all(&dst)?;
        for src_entry in fs::read_dir(src)? {
            let src_entry = src_entry?;
            let dst_path = dst.as_ref().join(src_entry.file_name());
            let entry_type = src_entry.file_type()?;
            if entry_type.is_dir() {
                copy_dir_all(src_entry.path(), dst_path)?;
            } else if entry_type.is_file() {
                fs::copy(src_entry.path(), dst_path)?;
            }
        }
        Ok(())
    }

    check_dir_all(&src, &dst)?;
    copy_all(src, dst)
}

fn auto_locate_template_dir(
    dir: &Path,
    prompt: impl Fn(&Variable) -> Result<String>,
) -> Result<PathBuf> {
    let configs = config::locate_template_configs(dir)?;
    match configs.len() {
        0 => Ok(dir.to_owned()),
        1 => Ok(dir.join(&configs[0])),
        _ => {
            let prompt_args = Variable {
                prompt: "Which template should be expanded?".into(),
                var_name: "Template".into(),
                var_info: VariableInfo::String {
                    entry: Box::new(StringEntry {
                        default: Some(configs[0].clone()),
                        choices: Some(configs),
                        regex: None,
                    }),
                },
            };
            let path = prompt(&prompt_args)?;
            Ok(dir.join(&path))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{auto_locate_template_dir, variable::VariableInfo};
    use anyhow::{anyhow, bail, Result};
    use std::{
        fs,
        io::Write,
        path::{Path, PathBuf},
    };
    use tempfile::{tempdir, TempDir};

    #[test]
    fn auto_locate_template_returns_base_when_no_spin_generate_is_found() -> Result<()> {
        let tmp = tempdir().unwrap();
        create_file(&tmp, "dir1/spin.toml", "")?;
        create_file(&tmp, "dir2/dir2_1/spin.toml", "")?;
        create_file(&tmp, "dir3/spin.toml", "")?;

        let r = auto_locate_template_dir(tmp.path(), |_slots| Err(anyhow!("test")))?;
        assert_eq!(tmp.path(), r);
        Ok(())
    }

    #[test]
    fn auto_locate_template_returns_path_when_single_spin_generate_is_found() -> Result<()> {
        let tmp = tempdir().unwrap();
        create_file(&tmp, "dir1/spin.toml", "")?;
        create_file(&tmp, "dir2/dir2_1/spin.toml", "")?;
        create_file(&tmp, "dir2/dir2_2/spin-generate.toml", "")?;
        create_file(&tmp, "dir3/spin.toml", "")?;

        let r = auto_locate_template_dir(tmp.path(), |_slots| Err(anyhow!("test")))?;
        assert_eq!(tmp.path().join("dir2/dir2_2"), r);
        Ok(())
    }

    #[test]
    fn auto_locate_template_prompts_when_multiple_spin_generate_is_found() -> Result<()> {
        let tmp = tempdir().unwrap();
        create_file(&tmp, "dir1/spin.toml", "")?;
        create_file(&tmp, "dir2/dir2_1/spin.toml", "")?;
        create_file(&tmp, "dir2/dir2_2/spin-generate.toml", "")?;
        create_file(&tmp, "dir3/spin.toml", "")?;
        create_file(&tmp, "dir4/spin-generate.toml", "")?;

        let r = auto_locate_template_dir(tmp.path(), |slots| match &slots.var_info {
            VariableInfo::Bool { .. } => bail!("Wrong prompt type"),
            VariableInfo::String { entry } => {
                if let Some(mut choices) = entry.choices.clone() {
                    choices.sort();
                    let expected = vec![
                        Path::new("dir2").join("dir2_2").to_string(),
                        "dir4".to_string(),
                    ];
                    assert_eq!(expected, choices);
                    Ok("my_path".to_string())
                } else {
                    bail!("Missing choices")
                }
            }
        });
        assert_eq!(tmp.path().join("my_path"), r?);
        Ok(())
    }

    pub trait PathString {
        fn to_string(&self) -> String;
    }

    impl PathString for PathBuf {
        fn to_string(&self) -> String {
            self.as_path().to_string()
        }
    }

    impl PathString for Path {
        fn to_string(&self) -> String {
            self.display().to_string()
        }
    }

    pub fn create_file(base_path: &TempDir, path: &str, contents: &str) -> anyhow::Result<()> {
        let path = base_path.path().join(path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::File::create(&path)?.write_all(contents.as_ref())?;
        Ok(())
    }
}
