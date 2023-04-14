mod host_component;

use std::collections::HashMap;

pub use host_component::SqliteComponent;
use rand::Rng;
use spin_core::{
    async_trait,
    sqlite::{self, Host},
};

pub struct SqliteImpl {
    connections: HashMap<sqlite::Connection, rusqlite::Connection>,
    statements: HashMap<sqlite::Statement, (String, Vec<String>)>,
}

impl SqliteImpl {
    pub fn new() -> Self {
        Self {
            connections: HashMap::default(),
            statements: HashMap::default(),
        }
    }

    pub fn component_init(&mut self) {}

    fn find_statement(
        &mut self,
        connection: sqlite::Connection,
        statement: sqlite::Statement,
    ) -> Result<(&mut rusqlite::Connection, (&str, &[String])), sqlite::Error> {
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
        name: String,
    ) -> anyhow::Result<Result<spin_core::sqlite::Connection, spin_core::sqlite::Error>> {
        println!("Opening..");
        let conn = rusqlite::Connection::open_in_memory()?;
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
        c.execute(s, rusqlite::params_from_iter(a.iter()))
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
            .query_map(rusqlite::params_from_iter(a.iter()), |row| {
                let mut values = vec![];
                for column in 0.. {
                    let value = match row.get_ref(column) {
                        Ok(rusqlite::types::ValueRef::Null) => sqlite::DataType::Null,
                        Ok(rusqlite::types::ValueRef::Integer(i)) => sqlite::DataType::Int64(i),
                        Ok(rusqlite::types::ValueRef::Real(f)) => sqlite::DataType::Double(f),
                        Ok(rusqlite::types::ValueRef::Text(t)) => {
                            sqlite::DataType::Str(String::from_utf8(t.to_vec()).unwrap())
                        }
                        Ok(rusqlite::types::ValueRef::Blob(b)) => {
                            sqlite::DataType::Binary(b.to_vec())
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
        params: Vec<String>,
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
