use spin_world::sqlite::{self, RowResult};

#[derive(Clone)]
pub struct LibsqlClient {
    inner: libsql_client::http::Client,
}

impl LibsqlClient {
    pub fn create(url: &str, token: String) -> anyhow::Result<Self> {
        let config = libsql_client::Config::new(url)?.with_auth_token(token);
        let inner = libsql_client::http::Client::from_config(
            libsql_client::http::InnerClient::Reqwest(libsql_client::reqwest::HttpClient::new()),
            config,
        )?;
        Ok(Self { inner })
    }
}

#[async_trait::async_trait]
impl spin_sqlite::Connection for LibsqlClient {
    async fn query(
        &self,
        query: &str,
        parameters: Vec<sqlite::Value>,
    ) -> Result<sqlite::QueryResult, sqlite::Error> {
        let stmt =
            libsql_client::statement::Statement::with_args(query, &convert_parameters(&parameters));
        let client = self.inner.clone();

        let result = client
            .execute(stmt)
            .await
            .map_err(|e| sqlite::Error::Io(e.to_string()))?;

        Ok(sqlite::QueryResult {
            columns: result.columns,
            rows: convert_rows(result.rows),
        })
    }

    async fn execute_batch(&self, statements: &str) -> anyhow::Result<()> {
        let client = libsql_client::Client::Http(self.inner.clone());

        // Unfortunately, the libsql library requires that the statements are already split
        // into individual statement strings which requires us to parse the supplied SQL string.
        let stmts = sqlparser::parser::Parser::parse_sql(
            &sqlparser::dialect::SQLiteDialect {},
            statements,
        )?
        .into_iter()
        .map(|st| st.to_string())
        .map(libsql_client::Statement::from);

        let _ = client.batch(stmts).await?;

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
