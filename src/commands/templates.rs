use anyhow::Result;
use comfy_table::Table;
use fermyon_templates::TemplatesManager;
use std::path::PathBuf;
use structopt::StructOpt;

/// Commands for working with WebAssembly component templates.
#[derive(StructOpt, Debug)]
pub enum TemplatesCommand {
    /// Add a template repository locally.
    Add(Add),

    /// List the template repositories configured.
    List(List),

    /// Generate a new project from a template.
    Generate(Generate),
}

impl TemplatesCommand {
    pub async fn run(self) -> Result<()> {
        match self {
            TemplatesCommand::Add(cmd) => cmd.run().await,
            TemplatesCommand::Generate(cmd) => cmd.run().await,
            TemplatesCommand::List(cmd) => cmd.run().await,
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
    pub git: String,

    /// The optional branch of the git repository.
    #[structopt(long = "branch")]
    pub branch: Option<String>,
}

impl Add {
    pub async fn run(self) -> Result<()> {
        let tm = TemplatesManager::default().await?;
        Ok(tm.add_repo(&self.name, &self.git, self.branch.as_deref())?)
    }
}

/// Generate a new project based on a template.
#[derive(StructOpt, Debug)]
pub struct Generate {
    /// The local templates repository.
    #[structopt(long = "repo")]
    pub repo: String,

    /// The name of the template.
    #[structopt(long = "template")]
    pub template: String,

    /// The destination where the template will be used.
    #[structopt(long = "path")]
    pub path: PathBuf,
}

impl Generate {
    pub async fn run(self) -> Result<()> {
        let tm = TemplatesManager::default().await?;
        tm.generate(&self.repo, &self.template, self.path).await
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
