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
        parameters: Vec<spin_world::sqlite::Value>,
    ) -> Result<spin_world::sqlite::QueryResult, spin_world::sqlite::Error> {
        let stmt =
            libsql_client::statement::Statement::with_args(query, &convert_parameters(&parameters));
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let result = self
                .client
                .execute(stmt)
                .await
                .map_err(|e| spin_world::sqlite::Error::Io(e.to_string()))?;
            let rows = result
                .rows
                .into_iter()
                .map(|r| {
                    let values = r
                        .values
                        .into_iter()
                        .map(|v| match v {
                            libsql_client::Value::Null => sqlite::Value::Null,
                            libsql_client::Value::Integer { value } => {
                                sqlite::Value::Integer(value as _)
                            }
                            libsql_client::Value::Float { value } => sqlite::Value::Real(value),
                            libsql_client::Value::Text { value } => sqlite::Value::Text(value),
                            libsql_client::Value::Blob { value } => sqlite::Value::Blob(value),
                        })
                        .collect();
                    RowResult { values }
                })
                .collect();
            Ok(spin_world::sqlite::QueryResult {
                columns: result.columns,
                rows,
            })
        })
    }
}

fn convert_parameters(parameters: &[spin_world::sqlite::Value]) -> Vec<libsql_client::Value> {
    parameters
        .iter()
        .map(|v| match v {
            sqlite::Value::Integer(value) => libsql_client::Value::Integer { value: *value },
            sqlite::Value::Real(value) => libsql_client::Value::Float { value: *value },
            sqlite::Value::Text(t) => libsql_client::Value::Text { value: t.clone() },
            sqlite::Value::Blob(b) => libsql_client::Value::Blob { value: b.clone() },
            sqlite::Value::Null => libsql_client::Value::Null,
        })
        .collect()
}
