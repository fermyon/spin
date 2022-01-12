use anyhow::Result;
use spin_templates::TemplatesManager;
use std::path::PathBuf;
use structopt::StructOpt;

/// Scaffold a new application locally based on a template.
#[derive(StructOpt, Debug)]
pub struct NewCommand {
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

impl NewCommand {
    pub async fn run(self) -> Result<()> {
        let tm = TemplatesManager::default().await?;
        tm.generate(&self.repo, &self.template, self.path).await
    }
}
