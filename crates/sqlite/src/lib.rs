mod host_component;

use spin_app::{async_trait, MetadataKey};
use spin_key_value::table;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

pub use host_component::SqliteComponent;

pub const DATABASES_KEY: MetadataKey<HashSet<String>> = MetadataKey::new("databases");

pub trait ConnectionManager: Send + Sync {
    fn get_connection(&self) -> Result<Arc<dyn Connection + 'static>, spin_world::sqlite::Error>;
}

/// A trait abstracting over operations to a SQLite database
pub trait Connection: Send + Sync {
    fn query(
        &self,
        query: &str,
        parameters: Vec<spin_world::sqlite::Value>,
    ) -> Result<spin_world::sqlite::QueryResult, spin_world::sqlite::Error>;

    fn execute_batch(
        &self,
        _statements: &str,
    ) -> Result<spin_world::sqlite::QueryResult, spin_world::sqlite::Error>;
}

/// An implementation of the SQLite host
pub struct SqliteDispatch {
    allowed_databases: HashSet<String>,
    connections: table::Table<Arc<dyn Connection>>,
    client_manager: HashMap<String, Arc<dyn ConnectionManager>>,
}

impl SqliteDispatch {
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
        connection: spin_world::sqlite::Connection,
    ) -> Result<&Arc<dyn Connection>, spin_world::sqlite::Error> {
        self.connections
            .get(connection)
            .ok_or(spin_world::sqlite::Error::InvalidConnection)
    }
}

#[async_trait]
impl spin_world::sqlite::Host for SqliteDispatch {
    async fn open(
        &mut self,
        database: String,
    ) -> anyhow::Result<Result<spin_world::sqlite::Connection, spin_world::sqlite::Error>> {
        Ok(tokio::task::block_in_place(|| {
            if !self.allowed_databases.contains(&database) {
                return Err(spin_world::sqlite::Error::AccessDenied);
            }
            self.connections
                .push(
                    self.client_manager
                        .get(&database)
                        .ok_or(spin_world::sqlite::Error::NoSuchDatabase)?
                        .get_connection()?,
                )
                .map_err(|()| spin_world::sqlite::Error::DatabaseFull)
        }))
    }

    async fn execute(
        &mut self,
        connection: spin_world::sqlite::Connection,
        query: String,
        parameters: Vec<spin_world::sqlite::Value>,
    ) -> anyhow::Result<Result<spin_world::sqlite::QueryResult, spin_world::sqlite::Error>> {
        Ok(tokio::task::block_in_place(|| {
            let conn = self.get_connection(connection)?;
            conn.query(&query, parameters)
        }))
    }

    async fn close(&mut self, connection: spin_world::sqlite::Connection) -> anyhow::Result<()> {
        let _ = self.connections.remove(connection);
        Ok(())
    }
}
