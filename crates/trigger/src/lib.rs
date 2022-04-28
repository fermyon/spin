use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait Trigger {
    async fn run(&self) -> Result<()>;
}
