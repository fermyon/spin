mod host_component;

use std::collections::{HashMap, HashSet};

use spin_core::{
    async_trait,
    sqlite::{self, Host},
};

pub use host_component::DatabaseLocation;
pub use host_component::SqliteComponent;

pub struct SqliteImpl {
    location: DatabaseLocation,
    connections: HashMap<sqlite::Connection, rusqlite::Connection>,
    next_conn_key: u32,
    allowed_databases: HashSet<String>,
}

impl SqliteImpl {
    pub fn new(location: DatabaseLocation) -> Self {
        Self {
            location,
            connections: HashMap::default(),
            next_conn_key: 0,
            allowed_databases: HashSet::new(),
        }
    }

    pub fn component_init(&mut self, allowed_databases: HashSet<String>) {
        self.allowed_databases = allowed_databases
    }
}

#[async_trait]
impl Host for SqliteImpl {
    async fn open(
        &mut self,
        database: String,
    ) -> anyhow::Result<Result<spin_core::sqlite::Connection, spin_core::sqlite::Error>> {
        Ok(async {
            if !self.allowed_databases.contains(&database) {
                return Err(sqlite::Error::AccessDenied);
            }
            // TODO: handle more than one database
            let conn = match &self.location {
                DatabaseLocation::InMemory => rusqlite::Connection::open_in_memory()
                    .map_err(|e| sqlite::Error::Io(e.to_string()))?,
                DatabaseLocation::Path(p) => {
                    rusqlite::Connection::open(p).map_err(|e| sqlite::Error::Io(e.to_string()))?
                }
            };

            loop {
                let key = self.next_conn_key;
                self.next_conn_key = self.next_conn_key.wrapping_add(1);
                if !self.connections.contains_key(&key) {
                    self.connections.insert(key, conn);
                    break Ok(key);
                }
            }
        }
        .await)
    }

    async fn execute(
        &mut self,
        connection: sqlite::Connection,
        statement: String,
        parameters: Vec<sqlite::Value>,
    ) -> anyhow::Result<Result<(), sqlite::Error>> {
        Ok(async move {
            let conn = self
                .connections
                .get(&connection)
                .ok_or_else(|| sqlite::Error::InvalidConnection)?;
            let mut statement = conn
                .prepare_cached(&statement)
                .map_err(|e| sqlite::Error::Io(e.to_string()))?;
            statement
                .execute(rusqlite::params_from_iter(convert_data(
                    parameters.into_iter(),
                )))
                .map_err(|e| sqlite::Error::Io(e.to_string()))?;
            Ok(())
        }
        .await)
    }

    async fn query(
        &mut self,
        connection: sqlite::Connection,
        query: String,
        parameters: Vec<sqlite::Value>,
    ) -> anyhow::Result<Result<Vec<sqlite::Row>, sqlite::Error>> {
        Ok(async move {
            let conn = self.connections.get(&connection).expect("TODO");
            let mut statement = conn
                .prepare_cached(&query)
                .map_err(|e| sqlite::Error::Io(e.to_string()))?;
            let rows = statement
                .query_map(
                    rusqlite::params_from_iter(convert_data(parameters.into_iter())),
                    |row| {
                        let mut values = vec![];
                        for column in 0.. {
                            let name = row.as_ref().column_name(column);
                            if let Err(rusqlite::Error::InvalidColumnIndex(_)) = name {
                                break;
                            }
                            let name = name?.to_string();
                            let value = row.get::<usize, ValueWrapper>(column);
                            if let Err(rusqlite::Error::InvalidColumnIndex(_)) = value {
                                break;
                            }
                            let value = value?.0;
                            values.push(sqlite::ColumnValue { name, value });
                        }
                        Ok(sqlite::Row { values })
                    },
                )
                .map_err(|e| sqlite::Error::Io(e.to_string()))?;
            Ok(rows
                .map(|r| r.map_err(|e| sqlite::Error::Io(e.to_string())))
                .collect::<Result<_, sqlite::Error>>()?)
        }
        .await)
    }

    async fn close(&mut self, connection: spin_core::sqlite::Connection) -> anyhow::Result<()> {
        Ok(async {
            let _ = self.connections.remove(&connection);
        }
        .await)
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
