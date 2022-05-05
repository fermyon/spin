use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use comfy_table::Table;

use spin_templates::{
    InstallationResults, ProgressReporter, SkippedReason, Template, TemplateManager, TemplateSource,
};

const INSTALL_FROM_DIR_OPT: &str = "FROM_DIR";
const INSTALL_FROM_GIT_OPT: &str = "FROM_GIT";

/// Commands for working with WebAssembly component templates.
#[derive(Subcommand, Debug)]
pub enum TemplateCommands {
    /// Install templates from a Git repository or local directory.
    Install(Install),

    /// Remove a template from your installation.
    Uninstall(Uninstall),

    /// List the installed templates.
    List(List),
}

impl TemplateCommands {
    pub async fn run(self) -> Result<()> {
        match self {
            TemplateCommands::Install(cmd) => cmd.run().await,
            TemplateCommands::Uninstall(cmd) => cmd.run().await,
            TemplateCommands::List(cmd) => cmd.run().await,
        }
    }
}

/// Install templates from a Git repository or local directory.
#[derive(Parser, Debug)]
pub struct Install {
    /// The URL of the templates git repository.
    /// The templates must be in a git repository in a "templates" directory.
    #[clap(
        name = INSTALL_FROM_GIT_OPT,
        long = "git",
        conflicts_with = INSTALL_FROM_DIR_OPT,
    )]
    pub git: Option<String>,

    /// The optional branch of the git repository.
    #[clap(long = "branch", requires = INSTALL_FROM_GIT_OPT)]
    pub branch: Option<String>,

    /// Local directory containing the template(s) to install.
    #[clap(
        name = INSTALL_FROM_DIR_OPT,
        long = "dir",
        conflicts_with = INSTALL_FROM_GIT_OPT,
    )]
    pub dir: Option<PathBuf>,
}

/// Remove a template from your installation.
#[derive(Parser, Debug)]
pub struct Uninstall {
    /// The template to uninstall.
    pub template_id: String,
}

impl Install {
    pub async fn run(self) -> Result<()> {
        let template_manager =
            TemplateManager::default().context("Failed to construct template directory path")?;
        let source = match (&self.git, &self.dir) {
            (Some(git), None) => TemplateSource::try_from_git(&git, &self.branch)?,
            (None, Some(dir)) => TemplateSource::File(dir.clone()),
            _ => anyhow::bail!("Exactly one of `git` and `dir` sources must be specified"),
        };

        let reporter = ConsoleProgressReporter;

        let installation_results = template_manager
            .install(&source, &reporter)
            .await
            .context("Failed to install one or more templates")?;

        self.print_installed_templates(&installation_results);

        Ok(())
    }

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
                    table.add_row(vec![id.clone(), reason_text(reason)]);
                }

                println!();
                println!("{}", table);
            }
        }
    }
}

impl Uninstall {
    pub async fn run(self) -> Result<()> {
        let template_manager =
            TemplateManager::default().context("Failed to construct template directory path")?;

        template_manager
            .uninstall(&self.template_id)
            .await
            .context("Failed to uninstall template")?;

        Ok(())
    }
}

/// List the installed templates.
#[derive(Parser, Debug)]
pub struct List {}

impl List {
    pub async fn run(self) -> Result<()> {
        let template_manager =
            TemplateManager::default().context("Failed to construct template directory path")?;
        let templates = template_manager
            .list()
            .await
            .context("Failed to list templates")?;

        self.print_templates(&templates);

        Ok(())
    }

    fn print_templates(&self, templates: &[Template]) {
        if templates.is_empty() {
            println!("You have no templates installed. Run");
            println!("spin templates install --git https://github.com/fermyon/spin");
            println!("to install a starter set.");
            println!();
        } else {
            let mut table = Table::new();
            table.set_header(vec!["Name", "Description"]);
            table.load_preset(comfy_table::presets::ASCII_BORDERS_ONLY_CONDENSED);

            for template in templates {
                table.add_row(vec![template.id(), template.description_or_empty()]);
            }

            println!("{}", table);
        }
    }
}

struct ConsoleProgressReporter;

impl ProgressReporter for ConsoleProgressReporter {
    fn report(&self, message: impl AsRef<str>) {
        println!("{}", message.as_ref());
    }
}

fn reason_text(reason: &SkippedReason) -> String {
    match reason {
        SkippedReason::AlreadyExists => "Already exists",
    }
    .to_owned()
}
