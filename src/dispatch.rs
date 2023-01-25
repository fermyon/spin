pub use async_trait::async_trait;
pub use anyhow::Result;

#[async_trait(?Send)]
pub trait Runner {
    async fn run(&self) -> Result<()>;
    async fn help(&self) -> Result<()>;
}

#[async_trait(?Send)]
pub trait Dispatch {
    async fn dispatch(&self, action: &Action) -> Result<()> {
        match action {
            Action::Run => self.run().await,
            Action::Help => self.help().await
        }
    }

    async fn run(&self) -> Result<()> {
        Ok(())
    }

    async fn help(&self) -> Result<()> {
        Ok(())
    }
}

pub enum Action {
    Run,
    Help
}

pub mod macros;