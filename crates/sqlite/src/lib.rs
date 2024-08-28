use spin_core::async_trait;
use spin_world::v2::sqlite;

/// A trait abstracting over operations to a SQLite database
#[async_trait]
pub trait Connection: Send + Sync {
    async fn query(
        &self,
        query: &str,
        parameters: Vec<sqlite::Value>,
    ) -> Result<sqlite::QueryResult, sqlite::Error>;

    async fn execute_batch(&self, statements: &str) -> anyhow::Result<()>;
}
