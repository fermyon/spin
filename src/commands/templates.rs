use std::path::PathBuf;

use anyhow::{Context, Result};
use async_trait::async_trait;
use clap::{Parser, Subcommand, ValueEnum};
use comfy_table::Table;
use path_absolutize::Absolutize;

use serde::Serialize;
use spin_templates::{
    InstallOptions, InstallationResults, InstalledTemplateWarning, ListResults, ProgressReporter,
    SkippedReason, Template, TemplateManager, TemplateSource,
};
use crate::dispatch::{Dispatch, Action};

const INSTALL_FROM_DIR_OPT: &str = "FROM_DIR";
const INSTALL_FROM_GIT_OPT: &str = "FROM_GIT";

const DEFAULT_TEMPLATES_INSTALL_PROMPT: &str =
    "You don't have any templates yet. Would you like to install the default set?";
const DEFAULT_TEMPLATE_REPO: &str = "https://github.com/fermyon/spin";

/// Commands for working with WebAssembly component templates.
#[derive(Subcommand, Debug)]
pub enum TemplateCommands {
    /// Install templates from a Git repository or local directory.
    ///
    /// The files of the templates are copied to the local template store: a
    /// directory in your data or home directory.
    Install(Install),

    /// Remove a template from your installation.
    Uninstall(Uninstall),

    /// List the installed templates.
    List(List),
}

#[async_trait(?Send)]
impl Dispatch for TemplateCommands {
    async fn dispatch(&self, action: &Action) -> Result<()> {
        match self {
            Self::Install(cmd) => cmd.dispatch(action).await,
            Self::Uninstall(cmd) => cmd.dispatch(action).await,
            Self::List(cmd) => cmd.dispatch(action).await,
        }
    }
}

/// Install templates from a Git repository or local directory.
#[derive(Parser, Debug)]
pub struct Install {
    /// The URL of the templates git repository.
    /// The templates must be in a git repository in a "templates" directory.
    #[arg(
        id = INSTALL_FROM_GIT_OPT,
        long = "git",
        conflicts_with = INSTALL_FROM_DIR_OPT,
    )]
    pub git: Option<String>,

    /// The optional branch of the git repository.
    #[arg(long, requires = INSTALL_FROM_GIT_OPT)]
    pub branch: Option<String>,

    /// Local directory containing the template(s) to install.
    #[arg(
        id = INSTALL_FROM_DIR_OPT,
        long = "dir",
        conflicts_with = INSTALL_FROM_GIT_OPT,
    )]
    pub dir: Option<PathBuf>,

    /// If present, updates existing templates instead of skipping.
    #[arg(long)]
    pub update: bool,
}

/// Remove a template from your installation.
#[derive(Parser, Debug)]
pub struct Uninstall {
    /// The template to uninstall.
    pub template_id: String,
}

#[async_trait(?Send)]
impl Dispatch for Install {
    async fn run(&self) -> Result<()> {
        let template_manager = TemplateManager::try_default()
            .context("Failed to construct template directory path")?;
        let source = match (&self.git, &self.dir) {
            (Some(git), None) => {
                TemplateSource::try_from_git(git, &self.branch, env!("VERGEN_BUILD_SEMVER"))?
            }
            (None, Some(dir)) => {
                let abs_dir = dir.absolutize().map(|d| d.to_path_buf());
                TemplateSource::File(abs_dir.unwrap_or_else(|_| dir.clone()))
            }
            _ => anyhow::bail!("Exactly one of `git` and `dir` sources must be specified"),
        };

        let reporter = ConsoleProgressReporter;
        let options = InstallOptions::default().update(self.update);

        let installation_results = template_manager
            .install(&source, &options, &reporter)
            .await
            .context("Failed to install one or more templates")?;

        self.print_installed_templates(&installation_results);

        Ok(())
    }
}

impl Install {
    fn print_installed_templates(&self, installation_results: &InstallationResults) {
        let templates = &installation_results.installed;
        let skipped = &installation_results.skipped;

        if templates.is_empty() && skipped.is_empty() {
            println!("The specified source contained no templates");
        } else {
            println!("Installed {} template(s)", templates.len());
            if !templates.is_empty() {
                let mut table = Table::new();
                table.set_header(vec!["Name", "Description"]);
                table.load_preset(comfy_table::presets::ASCII_BORDERS_ONLY_CONDENSED);

                for template in templates {
                    table.add_row(vec![template.id(), template.description_or_empty()]);
                }

                println!();
                println!("{}", table);
            }
            if !skipped.is_empty() {
                println!();
                println!("Skipped {} template(s)", skipped.len());

                let mut table = Table::new();
                table.set_header(vec!["Name", "Reason skipped"]);
                table.load_preset(comfy_table::presets::ASCII_BORDERS_ONLY_CONDENSED);

                for (id, reason) in skipped {
                    table.add_row(vec![id.clone(), skipped_reason_text(reason)]);
                }

                println!();
                println!("{}", table);
            }
        }
    }
}

#[async_trait(?Send)]
impl Dispatch for Uninstall {
    async fn run(&self) -> Result<()> {
        let template_manager = TemplateManager::try_default()
            .context("Failed to construct template directory path")?;

        template_manager
            .uninstall(&self.template_id)
            .await
            .context("Failed to uninstall template")?;

        Ok(())
    }
}

/// List the installed templates.
#[derive(Parser, Debug)]
pub struct List {
    /// The format in which to list the templates.
    #[arg(value_enum, long, default_value = "table", hide = true)]
    pub format: ListFormat,

    /// Whether to show additional template details in the list.
    #[arg(long)]
    pub verbose: bool,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum ListFormat {
    Table,
    Json,
}

#[async_trait(?Send)]
impl Dispatch for List {
    async fn run(&self) -> Result<()> {
        let template_manager = TemplateManager::try_default()
            .context("Failed to construct template directory path")?;
        let list_results = template_manager
            .list()
            .await
            .context("Failed to list templates")?;

        match (&self.format, list_results.templates.is_empty()) {
            (ListFormat::Table, false) => self.print_templates_table(&list_results),
            (ListFormat::Table, true) => {
                prompt_install_default_templates(&template_manager).await?;
            }
            (ListFormat::Json, _) => self.print_templates_json(&list_results)?,
        };

        Ok(())
    }
}

impl List {
    fn print_templates_table(&self, list_results: &ListResults) {
        let templates = &list_results.templates;
        let warnings = &list_results.warnings;
        if templates.is_empty() {
            println!();
        } else {
            let mut table = Table::new();

            let mut header = vec!["Name", "Description"];
            if self.verbose {
                header.push("Installed from");
            }

            table.set_header(header);
            table.load_preset(comfy_table::presets::ASCII_BORDERS_ONLY_CONDENSED);

            for template in templates {
                let mut row = vec![template.id(), template.description_or_empty()];
                if self.verbose {
                    row.push(template.installed_from_or_empty());
                }
                table.add_row(row);
            }

            println!("{}", table);
        }

        if !warnings.is_empty() {
            if !templates.is_empty() {
                println!();
            }

            for (id, warning) in warnings {
                println!(
                    "note: ignored invalid entry {} ({})",
                    id,
                    list_warn_reason_text(warning)
                );
            }
        }
    }

    fn print_templates_json(&self, list_results: &ListResults) -> anyhow::Result<()> {
        let json_vals: Vec<_> = list_results
            .templates
            .iter()
            .map(json_list_format)
            .collect();
        let json_text = serde_json::to_string_pretty(&json_vals)?;
        println!("{}", json_text);
        Ok(())
    }
}

fn json_list_format(template: &Template) -> TemplateListJson {
    TemplateListJson {
        id: template.id().to_owned(),
        description: template.description().as_ref().cloned(),
    }
}

#[derive(Serialize)]
struct TemplateListJson {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

struct ConsoleProgressReporter;

impl ProgressReporter for ConsoleProgressReporter {
    fn report(&self, message: impl AsRef<str>) {
        println!("{}", message.as_ref());
    }
}

fn skipped_reason_text(reason: &SkippedReason) -> String {
    match reason {
        SkippedReason::AlreadyExists => "Already exists".to_owned(),
        SkippedReason::InvalidManifest(msg) => format!("Template load error: {}", msg),
    }
}

fn list_warn_reason_text(reason: &InstalledTemplateWarning) -> String {
    match reason {
        InstalledTemplateWarning::InvalidManifest(msg) => format!("Template load error: {}", msg),
    }
}

pub(crate) async fn prompt_install_default_templates(
    template_manager: &TemplateManager,
) -> anyhow::Result<Option<Vec<Template>>> {
    let should_install = dialoguer::Confirm::new()
        .with_prompt(DEFAULT_TEMPLATES_INSTALL_PROMPT)
        .default(true)
        .interact_opt()?;
    if should_install == Some(true) {
        install_default_templates().await?;
        Ok(Some(template_manager.list().await?.templates))
    } else {
        println!(
            "You can install the default templates later with 'spin templates install --git {}'",
            DEFAULT_TEMPLATE_REPO
        );
        Ok(None)
    }
}

async fn install_default_templates() -> anyhow::Result<()> {
    let install_cmd = Install {
        git: Some(DEFAULT_TEMPLATE_REPO.to_owned()),
        branch: None,
        dir: None,
        update: false,
    };
    install_cmd
        .run()
        .await
        .context("Failed to install the default templates")?;
    Ok(())
}
