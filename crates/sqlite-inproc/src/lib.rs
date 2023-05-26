use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use once_cell::sync::OnceCell;
use spin_sqlite::{Connection, ConnectionManager};
use spin_world::sqlite;

#[derive(Debug, Clone)]
pub enum InProcDatabaseLocation {
    InMemory,
    Path(PathBuf),
}

/// A connection to a sqlite database
pub struct InProcConnectionManager {
    location: InProcDatabaseLocation,
    connection: OnceCell<Arc<dyn Connection>>,
}

impl InProcConnectionManager {
    pub fn new(location: InProcDatabaseLocation) -> Self {
        Self {
            location,
            connection: OnceCell::new(),
        }
    }
}

impl ConnectionManager for InProcConnectionManager {
    fn get_connection(&self) -> Result<Arc<dyn Connection>, spin_world::sqlite::Error> {
        let connection = self
            .connection
            .get_or_try_init(|| -> Result<_, sqlite::Error> {
                let c = match &self.location {
                    InProcDatabaseLocation::InMemory => rusqlite::Connection::open_in_memory(),
                    InProcDatabaseLocation::Path(path) => rusqlite::Connection::open(path),
                }
                .map_err(|e| sqlite::Error::Io(e.to_string()))?;
                Ok(Arc::new(InProcConnection(Mutex::new(c))))
            })?
            .clone();
        Ok(connection)
    }
}

struct InProcConnection(Mutex<rusqlite::Connection>);

impl Connection for InProcConnection {
    fn query(
        &self,
        query: &str,
        parameters: Vec<spin_world::sqlite::Value>,
    ) -> Result<spin_world::sqlite::QueryResult, spin_world::sqlite::Error> {
        let conn = self.0.lock().unwrap();
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

// A wrapper around sqlite::Value so that we can convert from rusqlite ValueRef
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
