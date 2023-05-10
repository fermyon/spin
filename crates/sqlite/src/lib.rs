mod host_component;

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, MutexGuard},
};

use rusqlite::Connection;
use spin_app::MetadataKey;
use spin_core::async_trait;
use spin_world::sqlite::{self, Host};

pub use host_component::{ConnectionManager, DatabaseLocation, SqliteComponent, SqliteConnection};
use spin_key_value::table;

pub const DATABASES_KEY: MetadataKey<HashSet<String>> = MetadataKey::new("databases");

pub struct SqliteImpl {
    allowed_databases: HashSet<String>,
    connections: table::Table<Arc<Mutex<Connection>>>,
    client_manager: HashMap<String, Arc<dyn ConnectionManager>>,
}

impl SqliteImpl {
    pub fn new(client_manager: HashMap<String, Arc<dyn ConnectionManager>>) -> Self {
        Self {
            connections: table::Table::new(256),
            allowed_databases: HashSet::new(),
            client_manager,
        }
    }

    pub fn component_init(&mut self, allowed_databases: HashSet<String>) {
        self.allowed_databases = allowed_databases
    }

    fn get_connection(
        &self,
        connection: sqlite::Connection,
    ) -> Result<MutexGuard<'_, Connection>, sqlite::Error> {
        Ok(self
            .connections
            .get(connection)
            .ok_or(sqlite::Error::InvalidConnection)?
            .lock()
            .unwrap())
    }
}

#[async_trait]
impl Host for SqliteImpl {
    async fn open(
        &mut self,
        database: String,
    ) -> anyhow::Result<Result<sqlite::Connection, sqlite::Error>> {
        Ok(tokio::task::block_in_place(|| {
            if !self.allowed_databases.contains(&database) {
                return Err(sqlite::Error::AccessDenied);
            }
            self.connections
                .push(
                    self.client_manager
                        .get(&database)
                        .ok_or(sqlite::Error::NoSuchDatabase)?
                        .get_connection()?,
                )
                .map_err(|()| sqlite::Error::DatabaseFull)
        }))
    }

    async fn execute(
        &mut self,
        connection: sqlite::Connection,
        statement: String,
        parameters: Vec<sqlite::Value>,
    ) -> anyhow::Result<Result<(), sqlite::Error>> {
        Ok(tokio::task::block_in_place(|| {
            let conn = self.get_connection(connection)?;
            let mut statement = conn
                .prepare_cached(&statement)
                .map_err(|e| sqlite::Error::Io(e.to_string()))?;
            statement
                .execute(rusqlite::params_from_iter(convert_data(
                    parameters.into_iter(),
                )))
                .map_err(|e| sqlite::Error::Io(e.to_string()))?;
            Ok(())
        }))
    }

    async fn query(
        &mut self,
        connection: sqlite::Connection,
        query: String,
        parameters: Vec<sqlite::Value>,
    ) -> anyhow::Result<Result<sqlite::QueryResult, sqlite::Error>> {
        Ok(tokio::task::block_in_place(|| {
            let conn = self.get_connection(connection)?;
            let mut statement = conn
                .prepare_cached(&query)
                .map_err(|e| sqlite::Error::Io(e.to_string()))?;
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
                        Ok(sqlite::RowResult { values })
                    },
                )
                .map_err(|e| sqlite::Error::Io(e.to_string()))?;
            let rows = rows
                .into_iter()
                .map(|r| r.map_err(|e| sqlite::Error::Io(e.to_string())))
                .collect::<Result<_, sqlite::Error>>()?;
            Ok(sqlite::QueryResult { columns, rows })
        }))
    }

    async fn close(&mut self, connection: sqlite::Connection) -> anyhow::Result<()> {
        let _ = self.connections.remove(connection);
        Ok(())
    }
}

// A wrapper around sqlite::Value so that we can convert from rusqlite ValueRef
struct ValueWrapper(sqlite::Value);

impl rusqlite::types::FromSql for ValueWrapper {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let value = match value {
            rusqlite::types::ValueRef::Null => sqlite::Value::Null,
            rusqlite::types::ValueRef::Integer(i) => sqlite::Value::Integer(i),
            rusqlite::types::ValueRef::Real(f) => sqlite::Value::Real(f),
            rusqlite::types::ValueRef::Text(t) => {
                sqlite::Value::Text(String::from_utf8(t.to_vec()).unwrap())
            }
            rusqlite::types::ValueRef::Blob(b) => sqlite::Value::Blob(b.to_vec()),
        };
        Ok(ValueWrapper(value))
    }
}

fn convert_data(
    arguments: impl Iterator<Item = sqlite::Value>,
) -> impl Iterator<Item = rusqlite::types::Value> {
    arguments.map(|a| match a {
        sqlite::Value::Null => rusqlite::types::Value::Null,
        sqlite::Value::Integer(i) => rusqlite::types::Value::Integer(i),
        sqlite::Value::Real(r) => rusqlite::types::Value::Real(r),
        sqlite::Value::Text(t) => rusqlite::types::Value::Text(t),
        sqlite::Value::Blob(b) => rusqlite::types::Value::Blob(b),
    })
}
