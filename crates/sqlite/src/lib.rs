mod host_component;

use spin_app::{async_trait, MetadataKey};
use spin_core::wasmtime::component::Resource;
use spin_key_value::table;
use spin_world::v2::sqlite;
use std::{collections::HashSet, sync::Arc};

pub use host_component::{LegacySqliteComponent, SqliteComponent};

pub const DATABASES_KEY: MetadataKey<HashSet<String>> = MetadataKey::new("databases");

/// A store of connections for all accessible databases for an application
#[async_trait]
pub trait ConnectionsStore: Send + Sync {
    /// Get a `Connection` for a specific database
    async fn get_connection(
        &self,
        database: &str,
    ) -> Result<Option<Arc<dyn Connection + 'static>>, sqlite::Error>;

    fn has_connection_for(&self, database: &str) -> bool;
}

/// A trait abstracting over operations to a SQLite database
#[async_trait]
pub trait Connection: Send + Sync {
    async fn query(
        &self,
        query: &str,
        parameters: Vec<sqlite::Value>,
    ) -> Result<sqlite::QueryResult, sqlite::Error>;

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
        connection: Resource<sqlite::Connection>,
    ) -> Result<&Arc<dyn Connection>, sqlite::Error> {
        self.connections
            .get(connection.rep())
            .ok_or(sqlite::Error::InvalidConnection)
    }
}

#[async_trait]
impl sqlite::Host for SqliteDispatch {}

#[async_trait]
impl sqlite::HostConnection for SqliteDispatch {
    async fn open(
        &mut self,
        database: String,
    ) -> anyhow::Result<Result<Resource<sqlite::Connection>, sqlite::Error>> {
        if !self.allowed_databases.contains(&database) {
            return Ok(Err(sqlite::Error::AccessDenied));
        }
        Ok(self
            .connections_store
            .get_connection(&database)
            .await
            .and_then(|conn| conn.ok_or(sqlite::Error::NoSuchDatabase))
            .and_then(|conn| {
                self.connections
                    .push(conn)
                    .map_err(|()| sqlite::Error::Io("too many connections opened".to_string()))
            })
            .map(Resource::new_own))
    }

    async fn execute(
        &mut self,
        connection: Resource<sqlite::Connection>,
        query: String,
        parameters: Vec<sqlite::Value>,
    ) -> anyhow::Result<Result<sqlite::QueryResult, sqlite::Error>> {
        let conn = match self.get_connection(connection) {
            Ok(c) => c,
            Err(err) => return Ok(Err(err)),
        };
        Ok(conn.query(&query, parameters).await)
    }

    fn drop(&mut self, connection: Resource<sqlite::Connection>) -> anyhow::Result<()> {
        let _ = self.connections.remove(connection.rep());
        Ok(())
    }
}

#[async_trait]
impl spin_world::v1::sqlite::Host for SqliteDispatch {
    async fn open(
        &mut self,
        database: String,
    ) -> anyhow::Result<Result<u32, spin_world::v1::sqlite::Error>> {
        let result = <Self as sqlite::HostConnection>::open(self, database).await?;
        Ok(result.map_err(to_legacy_error).map(|s| {
            let rep = s.rep();
            std::mem::forget(s);
            rep
        }))
    }

    async fn execute(
        &mut self,
        connection: u32,
        query: String,
        parameters: Vec<spin_world::v1::sqlite::Value>,
    ) -> anyhow::Result<Result<spin_world::v1::sqlite::QueryResult, spin_world::v1::sqlite::Error>>
    {
        let this = Resource::new_borrow(connection);
        let result = <Self as sqlite::HostConnection>::execute(
            self,
            this,
            query,
            parameters.into_iter().map(from_legacy_value).collect(),
        )
        .await?;
        Ok(result.map_err(to_legacy_error).map(to_legacy_query_result))
    }

    async fn close(&mut self, connection: u32) -> anyhow::Result<()> {
        <Self as sqlite::HostConnection>::drop(self, Resource::new_own(connection))
    }
}
use spin_world::v1::sqlite as v1;

fn to_legacy_error(error: sqlite::Error) -> v1::Error {
    match error {
        sqlite::Error::NoSuchDatabase => v1::Error::NoSuchDatabase,
        sqlite::Error::AccessDenied => v1::Error::AccessDenied,
        sqlite::Error::InvalidConnection => v1::Error::InvalidConnection,
        sqlite::Error::DatabaseFull => v1::Error::DatabaseFull,
        sqlite::Error::Io(s) => v1::Error::Io(s),
    }
}

fn to_legacy_query_result(result: sqlite::QueryResult) -> v1::QueryResult {
    v1::QueryResult {
        columns: result.columns,
        rows: result.rows.into_iter().map(to_legacy_row_result).collect(),
    }
}

fn to_legacy_row_result(result: sqlite::RowResult) -> v1::RowResult {
    v1::RowResult {
        values: result.values.into_iter().map(to_legacy_value).collect(),
    }
}

fn to_legacy_value(value: sqlite::Value) -> v1::Value {
    match value {
        sqlite::Value::Integer(i) => v1::Value::Integer(i),
        sqlite::Value::Real(r) => v1::Value::Real(r),
        sqlite::Value::Text(t) => v1::Value::Text(t),
        sqlite::Value::Blob(b) => v1::Value::Blob(b),
        sqlite::Value::Null => v1::Value::Null,
    }
}

fn from_legacy_value(value: v1::Value) -> sqlite::Value {
    match value {
        v1::Value::Integer(i) => sqlite::Value::Integer(i),
        v1::Value::Real(r) => sqlite::Value::Real(r),
        v1::Value::Text(t) => sqlite::Value::Text(t),
        v1::Value::Blob(b) => sqlite::Value::Blob(b),
        v1::Value::Null => sqlite::Value::Null,
    }
}
