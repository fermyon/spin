use anyhow::Context;
use async_trait::async_trait;
use spin_factor_sqlite::Connection;
use spin_world::v2::sqlite as v2;
use spin_world::v2::sqlite::{self, RowResult};
use tokio::sync::OnceCell;
use tracing::{instrument, Level};

/// A wrapper around a libSQL connection that implements the [`Connection`] trait.
pub struct LibSqlConnection {
    url: String,
    token: String,
    // Since the libSQL client can only be created asynchronously, we wait until
    // we're in the `Connection` implementation to create. Since we only want to do
    // this once, we use a `OnceCell` to store it.
    inner: OnceCell<LibsqlClient>,
}

impl LibSqlConnection {
    pub fn new(url: String, token: String) -> Self {
        Self {
            url,
            token,
            inner: OnceCell::new(),
        }
    }

    pub async fn get_client(&self) -> Result<&LibsqlClient, v2::Error> {
        self.inner
            .get_or_try_init(|| async {
                LibsqlClient::create(self.url.clone(), self.token.clone())
                    .await
                    .context("failed to create SQLite client")
            })
            .await
            .map_err(|_| v2::Error::InvalidConnection)
    }
}

#[async_trait]
impl Connection for LibSqlConnection {
    async fn query(
        &self,
        query: &str,
        parameters: Vec<v2::Value>,
    ) -> Result<v2::QueryResult, v2::Error> {
        let client = self.get_client().await?;
        client.query(query, parameters).await
    }

    async fn execute_batch(&self, statements: &str) -> anyhow::Result<()> {
        let client = self.get_client().await?;
        client.execute_batch(statements).await
    }

    fn summary(&self) -> Option<String> {
        Some(format!("libSQL at {}", self.url))
    }
}

#[derive(Clone)]
pub struct LibsqlClient {
    inner: libsql::Connection,
}

impl LibsqlClient {
    #[instrument(name = "spin_sqlite_libsql.create_connection", skip(token), err(level = Level::INFO), fields(otel.kind = "client", db.system = "sqlite"))]
    pub async fn create(url: String, token: String) -> anyhow::Result<Self> {
        let db = libsql::Builder::new_remote(url, token).build().await?;
        let inner = db.connect()?;
        Ok(Self { inner })
    }
}

impl LibsqlClient {
    #[instrument(name = "spin_sqlite_libsql.query", skip(self), err(level = Level::INFO), fields(otel.kind = "client", db.system = "sqlite", otel.name = query))]
    pub async fn query(
        &self,
        query: &str,
        parameters: Vec<sqlite::Value>,
    ) -> Result<sqlite::QueryResult, sqlite::Error> {
        let result = self
            .inner
            .query(query, convert_parameters(&parameters))
            .await
            .map_err(|e| sqlite::Error::Io(e.to_string()))?;

        Ok(sqlite::QueryResult {
            columns: columns(&result),
            rows: convert_rows(result)
                .await
                .map_err(|e| sqlite::Error::Io(e.to_string()))?,
        })
    }

    #[instrument(name = "spin_sqlite_libsql.execute_batch", skip(self), err(level = Level::INFO), fields(otel.kind = "client", db.system = "sqlite", db.statements = statements))]
    pub async fn execute_batch(&self, statements: &str) -> anyhow::Result<()> {
        self.inner.execute_batch(statements).await?;

        Ok(())
    }
}

fn columns(rows: &libsql::Rows) -> Vec<String> {
    (0..rows.column_count())
        .map(|index| rows.column_name(index).unwrap_or("").to_owned())
        .collect()
}

async fn convert_rows(mut rows: libsql::Rows) -> anyhow::Result<Vec<RowResult>> {
    let mut result_rows = vec![];

    let column_count = rows.column_count();

    while let Some(row) = rows.next().await? {
        result_rows.push(convert_row(row, column_count));
    }

    Ok(result_rows)
}

fn convert_row(row: libsql::Row, column_count: i32) -> RowResult {
    let values = (0..column_count)
        .map(|index| convert_value(row.get_value(index).unwrap()))
        .collect();
    RowResult { values }
}

fn convert_value(v: libsql::Value) -> sqlite::Value {
    use libsql::Value;

    match v {
        Value::Null => sqlite::Value::Null,
        Value::Integer(value) => sqlite::Value::Integer(value),
        Value::Real(value) => sqlite::Value::Real(value),
        Value::Text(value) => sqlite::Value::Text(value),
        Value::Blob(value) => sqlite::Value::Blob(value),
    }
}

fn convert_parameters(parameters: &[sqlite::Value]) -> Vec<libsql::Value> {
    use libsql::Value;

    parameters
        .iter()
        .map(|v| match v {
            sqlite::Value::Integer(value) => Value::Integer(*value),
            sqlite::Value::Real(value) => Value::Real(*value),
            sqlite::Value::Text(t) => Value::Text(t.clone()),
            sqlite::Value::Blob(b) => Value::Blob(b.clone()),
            sqlite::Value::Null => Value::Null,
        })
        .collect()
}
