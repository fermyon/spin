use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use anyhow::Context;
use async_trait::async_trait;
use spin_sqlite::Connection;

#[derive(Debug, Clone)]
pub enum InProcDatabaseLocation {
    InMemory,
    Path(PathBuf),
}

/// A connection to a sqlite database
pub struct InProcConnection {
    connection: Arc<Mutex<rusqlite::Connection>>,
}

impl InProcConnection {
    pub fn new(location: InProcDatabaseLocation) -> Result<Self, spin_world::sqlite::Error> {
        unsafe {
            rusqlite::ffi::sqlite3_auto_extension(Some(sqlite_vss::sqlite3_vector_init));
            rusqlite::ffi::sqlite3_auto_extension(Some(sqlite_vss::sqlite3_vss_init));
        }
        let connection = {
            let c = match &location {
                InProcDatabaseLocation::InMemory => rusqlite::Connection::open_in_memory(),
                InProcDatabaseLocation::Path(path) => rusqlite::Connection::open(path),
            }
            .map_err(|e| spin_world::sqlite::Error::Io(e.to_string()))?;
            Arc::new(Mutex::new(c))
        };
        Ok(Self { connection })
    }
}

#[async_trait]
impl Connection for InProcConnection {
    async fn query(
        &self,
        query: &str,
        parameters: Vec<spin_world::sqlite::Value>,
    ) -> Result<spin_world::sqlite::QueryResult, spin_world::sqlite::Error> {
        let connection = self.connection.clone();
        let query = query.to_owned();
        // Tell the tokio runtime that we're going to block while making the query
        tokio::task::spawn_blocking(move || execute_query(&connection, &query, parameters))
            .await
            .context("internal runtime error")
            .map_err(|e| spin_world::sqlite::Error::Io(e.to_string()))?
    }

    async fn execute_batch(&self, statements: &str) -> anyhow::Result<()> {
        let connection = self.connection.clone();
        let statements = statements.to_owned();
        tokio::task::spawn_blocking(move || {
            let conn = connection.lock().unwrap();
            conn.execute_batch(&statements)
                .context("failed to execute batch statements")
        })
        .await?
        .context("failed to spawn blocking task")?;
        Ok(())
    }
}

fn execute_query(
    connection: &Mutex<rusqlite::Connection>,
    query: &str,
    parameters: Vec<spin_world::sqlite::Value>,
) -> Result<spin_world::sqlite::QueryResult, spin_world::sqlite::Error> {
    let conn = connection.lock().unwrap();
    let mut statement = conn
        .prepare_cached(query)
        .map_err(|e| spin_world::sqlite::Error::Io(e.to_string()))?;
    let columns = statement
        .column_names()
        .into_iter()
        .map(ToOwned::to_owned)
        .collect();
    let rows = statement
        .query_map(
            rusqlite::params_from_iter(convert_data(parameters.into_iter())),
            |row| {
                let mut values = vec![];
                for column in 0.. {
                    let value = row.get::<usize, ValueWrapper>(column);
                    if let Err(rusqlite::Error::InvalidColumnIndex(_)) = value {
                        break;
                    }
                    let value = value?.0;
                    values.push(value);
                }
                Ok(spin_world::sqlite::RowResult { values })
            },
        )
        .map_err(|e| spin_world::sqlite::Error::Io(e.to_string()))?;
    let rows = rows
        .into_iter()
        .map(|r| r.map_err(|e| spin_world::sqlite::Error::Io(e.to_string())))
        .collect::<Result<_, spin_world::sqlite::Error>>()?;
    Ok(spin_world::sqlite::QueryResult { columns, rows })
}

fn convert_data(
    arguments: impl Iterator<Item = spin_world::sqlite::Value>,
) -> impl Iterator<Item = rusqlite::types::Value> {
    arguments.map(|a| match a {
        spin_world::sqlite::Value::Null => rusqlite::types::Value::Null,
        spin_world::sqlite::Value::Integer(i) => rusqlite::types::Value::Integer(i),
        spin_world::sqlite::Value::Real(r) => rusqlite::types::Value::Real(r),
        spin_world::sqlite::Value::Text(t) => rusqlite::types::Value::Text(t),
        spin_world::sqlite::Value::Blob(b) => rusqlite::types::Value::Blob(b),
    })
}

// A wrapper around spin_world::sqlite::Value so that we can convert from rusqlite ValueRef
struct ValueWrapper(spin_world::sqlite::Value);

impl rusqlite::types::FromSql for ValueWrapper {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let value = match value {
            rusqlite::types::ValueRef::Null => spin_world::sqlite::Value::Null,
            rusqlite::types::ValueRef::Integer(i) => spin_world::sqlite::Value::Integer(i),
            rusqlite::types::ValueRef::Real(f) => spin_world::sqlite::Value::Real(f),
            rusqlite::types::ValueRef::Text(t) => {
                spin_world::sqlite::Value::Text(String::from_utf8(t.to_vec()).unwrap())
            }
            rusqlite::types::ValueRef::Blob(b) => spin_world::sqlite::Value::Blob(b.to_vec()),
        };
        Ok(ValueWrapper(value))
    }
}
