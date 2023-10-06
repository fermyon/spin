mod host_component;

use spin_app::{async_trait, MetadataKey};
use spin_core::wasmtime::component::Resource;
use spin_key_value::table;
use std::{collections::HashSet, sync::Arc};

pub use host_component::SqliteComponent;

pub const DATABASES_KEY: MetadataKey<HashSet<String>> = MetadataKey::new("databases");

/// A store of connections for all accessible databases for an application
#[async_trait]
pub trait ConnectionsStore: Send + Sync {
    /// Get a `Connection` for a specific database
    async fn get_connection(
        &self,
        database: &str,
    ) -> Result<Option<Arc<dyn Connection + 'static>>, spin_world::sqlite::Error>;

    fn has_connection_for(&self, database: &str) -> bool;
}

/// A trait abstracting over operations to a SQLite database
#[async_trait]
pub trait Connection: Send + Sync {
    async fn query(
        &self,
        query: &str,
        parameters: Vec<spin_world::sqlite::Value>,
    ) -> Result<spin_world::sqlite::QueryResult, spin_world::sqlite::Error>;

    async fn execute_batch(&self, statements: &str) -> anyhow::Result<()>;
}

/// An implementation of the SQLite host
pub struct SqliteDispatch {
    allowed_databases: HashSet<String>,
    connections: table::Table<Arc<dyn Connection>>,
    connections_store: Arc<dyn ConnectionsStore>,
}

impl SqliteDispatch {
    pub fn new(connections_store: Arc<dyn ConnectionsStore>) -> Self {
        Self {
            connections: table::Table::new(256),
            allowed_databases: HashSet::new(),
            connections_store,
        }
    }

    /// (Re-)initialize dispatch for a give app
    pub fn component_init(
        &mut self,
        allowed_databases: HashSet<String>,
        connections_store: Arc<dyn ConnectionsStore>,
    ) {
        self.allowed_databases = allowed_databases;
        self.connections_store = connections_store;
    }

    fn get_connection(
        &self,
        connection: Resource<spin_world::sqlite::Connection>,
    ) -> Result<&Arc<dyn Connection>, spin_world::sqlite::Error> {
        self.connections
            .get(connection.rep())
            .ok_or(spin_world::sqlite::Error::InvalidConnection)
    }
}

#[async_trait]
impl spin_world::sqlite::Host for SqliteDispatch {}

#[async_trait]
impl spin_world::sqlite::HostConnection for SqliteDispatch {
    async fn open(
        &mut self,
        database: String,
    ) -> anyhow::Result<Result<Resource<spin_world::sqlite::Connection>, spin_world::sqlite::Error>>
    {
        if !self.allowed_databases.contains(&database) {
            return Ok(Err(spin_world::sqlite::Error::AccessDenied));
        }
        Ok(self
            .connections_store
            .get_connection(&database)
            .await
            .and_then(|conn| conn.ok_or(spin_world::sqlite::Error::NoSuchDatabase))
            .and_then(|conn| {
                self.connections.push(conn).map_err(|()| {
                    spin_world::sqlite::Error::Io("too many connections opened".to_string())
                })
            })
            .map(Resource::new_own))
    }

    async fn execute(
        &mut self,
        connection: Resource<spin_world::sqlite::Connection>,
        query: String,
        parameters: Vec<spin_world::sqlite::Value>,
    ) -> anyhow::Result<Result<spin_world::sqlite::QueryResult, spin_world::sqlite::Error>> {
        let conn = match self.get_connection(connection) {
            Ok(c) => c,
            Err(err) => return Ok(Err(err)),
        };
        Ok(conn.query(&query, parameters).await)
    }

    fn drop(&mut self, connection: Resource<spin_world::sqlite::Connection>) -> anyhow::Result<()> {
        let _ = self.connections.remove(connection.rep());
        Ok(())
    }
}
