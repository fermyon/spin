use anyhow::Result;
use comfy_table::Table;
use spin_templates::TemplatesManager;
use std::path::PathBuf;
use structopt::StructOpt;

/// Commands for working with WebAssembly component templates.
#[derive(StructOpt, Debug)]
pub enum TemplateCommands {
    /// Add a template repository locally.
    Add(Add),

    /// List the template repositories configured.
    List(List),
}

impl TemplateCommands {
    pub async fn run(self) -> Result<()> {
        match self {
            TemplateCommands::Add(cmd) => cmd.run().await,
            TemplateCommands::List(cmd) => cmd.run().await,
        }
    }
}

/// Add a templates repository from a remote git URL.
#[derive(StructOpt, Debug)]
pub struct Add {
    /// The name of the templates repository.
    #[structopt(long = "name")]
    pub name: String,

    /// The URL of the templates git repository.
    /// The templates must be in a git repository in a "templates" directory.
    #[structopt(long = "git")]
    pub git: Option<String>,

    /// The optional branch of the git repository.
    #[structopt(long = "branch")]
    pub branch: Option<String>,

    /// Local directory to add as a template.
    #[structopt(long = "local")]
    pub local: Option<PathBuf>,
}

impl Add {
    pub async fn run(self) -> Result<()> {
        let tm = TemplatesManager::default().await?;

        match self.git {
            Some(g) => Ok(tm.add_repo(&self.name, &g, self.branch.as_deref())?),
            None => Ok(tm.add_local(&self.name, &self.local.unwrap())?),
        }
    }
}

/// List existing templates.
#[derive(StructOpt, Debug)]
pub struct List {}

impl List {
    pub async fn run(self) -> Result<()> {
        let tm = TemplatesManager::default().await?;
        let res = tm.list().await?;
        let mut table = Table::new();
        table.set_header(vec!["Name", "Repository", "URL", "Branch"]);
        table.load_preset(comfy_table::presets::ASCII_BORDERS_ONLY_CONDENSED);

        for repo in res {
            for t in repo.clone().templates {
                table.add_row(vec![
                    t,
                    repo.clone().name,
                    repo.clone().git.unwrap_or("".to_string()),
                    repo.clone().branch.unwrap_or("".to_string()),
                ]);
            }
        }

        println!("{}", table);

        Ok(())
    }
}
