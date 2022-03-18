use anyhow::Result;
use spin_templates::{TemplateArgs, TemplatesManager};
use structopt::StructOpt;

/// Create a new application based on a template.
#[derive(StructOpt, Debug)]
pub struct NewCommand {
    #[structopt(flatten)]
    pub args: TemplateArgs,
}

impl NewCommand {
    pub async fn run(self) -> Result<()> {
        let tm = TemplatesManager::default().await?;
        tm.generate(self.args).await?;
        Ok(())
    }
}
