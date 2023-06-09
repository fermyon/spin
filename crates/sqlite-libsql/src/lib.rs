use std::sync::Arc;

use libsql_client::DatabaseClient;
use spin_world::sqlite::{self, RowResult};

#[derive(Clone)]
pub struct LibsqlClient {
    client: libsql_client::reqwest::Client,
}

impl LibsqlClient {
    pub fn new(url: String, token: String) -> Self {
        Self {
            client: libsql_client::reqwest::Client::new(url, token),
        }
    }
}

impl spin_sqlite::ConnectionManager for LibsqlClient {
    fn get_connection(&self) -> Result<Arc<dyn spin_sqlite::Connection>, sqlite::Error> {
        Ok(Arc::new(self.clone()))
    }
}

impl spin_sqlite::Connection for LibsqlClient {
    fn query(
        &self,
        query: &str,
        parameters: Vec<sqlite::Value>,
    ) -> Result<sqlite::QueryResult, sqlite::Error> {
        let stmt =
            libsql_client::statement::Statement::with_args(query, &convert_parameters(&parameters));
        let client = self.client.clone();

        // It's a bit buried under thread and async shenanigans, but this stanza
        // just calls libsql's `Client::execute(Statement)` function (and maps the
        // error case). (It is tricky to make a function to name it, though, because
        // of Send constraints.)
        let result = std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(client.execute(stmt))
        })
        .join()
        .unwrap_or_else(|_| Err(anyhow::anyhow!("internal thread error")))
        .map_err(|e| sqlite::Error::Io(e.to_string()))?;

        Ok(sqlite::QueryResult {
            columns: result.columns,
            rows: convert_rows(result.rows),
        })
    }

    fn execute_batch(&self, statements: &str) -> anyhow::Result<()> {
        let client = self.client.clone();

        // Unfortunately, the libsql library requires that the statements are already split
        // into individual statement strings which requires us to parse the supplied SQL string.
        let stmts: Vec<_> = sqlparser::parser::Parser::parse_sql(
            &sqlparser::dialect::SQLiteDialect {},
            statements,
        )?
        .iter()
        .map(|st| st.to_string())
        .map(libsql_client::Statement::from)
        .collect();

        // As in `query`, the shenanigans just wrap a call to libsql's `Client::batch()`.
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(client.batch(stmts))
        })
        .join()
        .unwrap_or_else(|_| Err(anyhow::anyhow!("internal thread error")))?;

        Ok(())
    }
}

fn convert_rows(rows: Vec<libsql_client::Row>) -> Vec<RowResult> {
    rows.into_iter()
        .map(|r| {
            let values = r.values.into_iter().map(convert_value).collect();
            RowResult { values }
        })
        .collect()
}

fn convert_value(v: libsql_client::Value) -> sqlite::Value {
    use libsql_client::Value;

    match v {
        Value::Null => sqlite::Value::Null,
        Value::Integer { value } => sqlite::Value::Integer(value),
        Value::Float { value } => sqlite::Value::Real(value),
        Value::Text { value } => sqlite::Value::Text(value),
        Value::Blob { value } => sqlite::Value::Blob(value),
    }
}

fn convert_parameters(parameters: &[sqlite::Value]) -> Vec<libsql_client::Value> {
    use libsql_client::Value;

    parameters
        .iter()
        .map(|v| match v {
            sqlite::Value::Integer(value) => Value::Integer { value: *value },
            sqlite::Value::Real(value) => Value::Float { value: *value },
            sqlite::Value::Text(t) => Value::Text { value: t.clone() },
            sqlite::Value::Blob(b) => Value::Blob { value: b.clone() },
            sqlite::Value::Null => Value::Null,
        })
        .collect()
}
