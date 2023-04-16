mod host_component;

use std::collections::HashMap;

use rand::Rng;
use spin_core::{
    async_trait,
    sqlite::{self, Host},
};

pub use host_component::DatabaseLocation;
pub use host_component::SqliteComponent;

pub struct SqliteImpl {
    location: DatabaseLocation,
    connections: HashMap<sqlite::Connection, rusqlite::Connection>,
    statements: HashMap<sqlite::Statement, (String, Vec<sqlite::DataType>)>,
}

impl SqliteImpl {
    pub fn new(location: DatabaseLocation) -> Self {
        Self {
            location,
            connections: HashMap::default(),
            statements: HashMap::default(),
        }
    }

    pub fn component_init(&mut self) {}

    fn find_statement(
        &mut self,
        connection: sqlite::Connection,
        statement: sqlite::Statement,
    ) -> Result<(&mut rusqlite::Connection, (&str, &[sqlite::DataType])), sqlite::Error> {
        match (
            self.connections.get_mut(&connection),
            self.statements.get(&statement),
        ) {
            (Some(c), Some((s, a))) => Ok((c, (s, a))),
            (Some(_), None) => todo!(),
            (None, _) => todo!(),
        }
    }
}

#[async_trait]
impl Host for SqliteImpl {
    async fn open(
        &mut self,
        _name: String,
    ) -> anyhow::Result<Result<spin_core::sqlite::Connection, spin_core::sqlite::Error>> {
        let conn = match &self.location {
            DatabaseLocation::InMemory => rusqlite::Connection::open_in_memory()?,
            DatabaseLocation::Path(p) => rusqlite::Connection::open(p)?,
        };

        // TODO: this is not the best way to do this...
        let mut rng = rand::thread_rng();
        let c: sqlite::Connection = rng.gen();
        self.connections.insert(c, conn);
        Ok(Ok(c))
    }

    async fn execute(
        &mut self,
        connection: sqlite::Connection,
        statement: sqlite::Statement,
    ) -> anyhow::Result<Result<(), sqlite::Error>> {
        let (c, (s, a)) = self.find_statement(connection, statement)?;
        c.execute(s, rusqlite::params_from_iter(convert_data(a.iter())))
            .map_err(|e| sqlite::Error::Io(e.to_string()))?;
        Ok(Ok(()))
    }

    async fn query(
        &mut self,
        connection: sqlite::Connection,
        query: sqlite::Statement,
    ) -> anyhow::Result<Result<Vec<sqlite::Row>, sqlite::Error>> {
        let (c, (q, a)) = self.find_statement(connection, query)?;
        let mut statement = c.prepare(q).map_err(|e| sqlite::Error::Io(e.to_string()))?;
        let rows = statement
            .query_map(rusqlite::params_from_iter(convert_data(a.iter())), |row| {
                let mut values = vec![];
                for column in 0.. {
                    let value = match row.get_ref(column) {
                        Ok(rusqlite::types::ValueRef::Null) => sqlite::DataType::Null,
                        Ok(rusqlite::types::ValueRef::Integer(i)) => sqlite::DataType::Integer(i),
                        Ok(rusqlite::types::ValueRef::Real(f)) => sqlite::DataType::Real(f),
                        Ok(rusqlite::types::ValueRef::Text(t)) => {
                            sqlite::DataType::Text(String::from_utf8(t.to_vec()).unwrap())
                        }
                        Ok(rusqlite::types::ValueRef::Blob(b)) => {
                            sqlite::DataType::Blob(b.to_vec())
                        }
                        Err(rusqlite::Error::InvalidColumnIndex(_)) => break,
                        _ => todo!(),
                    };
                    values.push(value);
                }
                Ok(sqlite::Row { values })
            })
            .map_err(|e| sqlite::Error::Io(e.to_string()))?;
        Ok(Ok(rows.collect::<Result<_, _>>()?))
    }

    async fn close(&mut self, connection: spin_core::sqlite::Connection) -> anyhow::Result<()> {
        let _ = self.connections.remove(&connection);
        Ok(())
    }

    async fn prepare_statement(
        &mut self,
        statement: String,
        params: Vec<sqlite::DataType>,
    ) -> anyhow::Result<Result<sqlite::Statement, sqlite::Error>> {
        let mut rng = rand::thread_rng();
        let s: sqlite::Statement = rng.gen();
        self.statements.insert(s, (statement, params));
        Ok(Ok(s))
    }

    async fn drop_statement(&mut self, statement: sqlite::Statement) -> anyhow::Result<()> {
        self.statements.remove(&statement);
        Ok(())
    }
}

fn convert_data<'a>(
    arguments: impl Iterator<Item = &'a sqlite::DataType> + 'a,
) -> impl Iterator<Item = rusqlite::types::Value> + 'a {
    arguments.map(|a| match a {
        sqlite::DataType::Null => rusqlite::types::Value::Null,
        sqlite::DataType::Integer(i) => rusqlite::types::Value::Integer(*i),
        sqlite::DataType::Real(r) => rusqlite::types::Value::Real(*r),
        sqlite::DataType::Text(t) => rusqlite::types::Value::Text(t.clone()),
        sqlite::DataType::Blob(b) => rusqlite::types::Value::Blob(b.clone()),
    })
}
