use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use async_trait::async_trait;
use spin_factors::wasmtime::component::Resource;
use spin_factors::{anyhow, Factor, RuntimeFactors, SelfInstanceBuilder};
use spin_locked_app::MetadataKey;
use spin_world::v1::sqlite as v1;
use spin_world::v2::sqlite as v2;

pub struct SqliteFactor {
    connections_store: Arc<dyn ConnectionsStore>,
}

impl SqliteFactor {
    pub fn new(connections_store: Arc<dyn ConnectionsStore>) -> Self {
        Self { connections_store }
    }
}

pub const ALLOWED_DATABASES_KEY: MetadataKey<Vec<String>> = MetadataKey::new("databases");

impl Factor for SqliteFactor {
    type RuntimeConfig = ();
    type AppState = AppState;
    type InstanceBuilder = InstanceState;

    fn init<T: RuntimeFactors>(
        &mut self,
        mut ctx: spin_factors::InitContext<T, Self>,
    ) -> anyhow::Result<()> {
        ctx.link_bindings(v1::add_to_linker)?;
        ctx.link_bindings(v2::add_to_linker)?;
        Ok(())
    }

    fn configure_app<T: spin_factors::RuntimeFactors>(
        &self,
        ctx: spin_factors::ConfigureAppContext<T, Self>,
    ) -> anyhow::Result<Self::AppState> {
        let allowed_databases = ctx
            .app()
            .components()
            .map(|component| {
                Ok((
                    component.id().to_string(),
                    component
                        .get_metadata(ALLOWED_DATABASES_KEY)?
                        .unwrap_or_default()
                        .into_iter()
                        .collect::<HashSet<_>>()
                        .into(),
                ))
            })
            .collect::<anyhow::Result<_>>()?;
        Ok(AppState { allowed_databases })
    }

    fn prepare<T: spin_factors::RuntimeFactors>(
        &self,
        ctx: spin_factors::PrepareContext<Self>,
        _builders: &mut spin_factors::InstanceBuilders<T>,
    ) -> spin_factors::anyhow::Result<Self::InstanceBuilder> {
        let allowed_databases = ctx
            .app_state()
            .allowed_databases
            .get(ctx.app_component().id())
            .cloned()
            .unwrap_or_default();
        Ok(InstanceState {
            connections: table::Table::new(256),
            allowed_databases,
            connections_store: self.connections_store.clone(),
        })
    }
}

pub struct AppState {
    allowed_databases: HashMap<String, Arc<HashSet<String>>>,
}

pub struct InstanceState {
    allowed_databases: Arc<HashSet<String>>,
    connections: table::Table<Arc<dyn Connection>>,
    connections_store: Arc<dyn ConnectionsStore>,
}

impl InstanceState {
    fn get_connection(
        &self,
        connection: Resource<v2::Connection>,
    ) -> Result<&Arc<dyn Connection>, v2::Error> {
        self.connections
            .get(connection.rep())
            .ok_or(v2::Error::InvalidConnection)
    }
}

impl SelfInstanceBuilder for InstanceState {}

impl v2::Host for InstanceState {
    fn convert_error(&mut self, error: v2::Error) -> anyhow::Result<v2::Error> {
        Ok(error)
    }
}

#[async_trait]
impl v2::HostConnection for InstanceState {
    async fn open(&mut self, database: String) -> Result<Resource<v2::Connection>, v2::Error> {
        if !self.allowed_databases.contains(&database) {
            return Err(v2::Error::AccessDenied);
        }
        self.connections_store
            .get_connection(&database)
            .await
            .and_then(|conn| conn.ok_or(v2::Error::NoSuchDatabase))
            .and_then(|conn| {
                self.connections
                    .push(conn)
                    .map_err(|()| v2::Error::Io("too many connections opened".to_string()))
            })
            .map(Resource::new_own)
    }

    async fn execute(
        &mut self,
        connection: Resource<v2::Connection>,
        query: String,
        parameters: Vec<v2::Value>,
    ) -> Result<v2::QueryResult, v2::Error> {
        let conn = match self.get_connection(connection) {
            Ok(c) => c,
            Err(err) => return Err(err),
        };
        conn.query(&query, parameters).await
    }

    fn drop(&mut self, connection: Resource<v2::Connection>) -> anyhow::Result<()> {
        let _ = self.connections.remove(connection.rep());
        Ok(())
    }
}

#[async_trait]
impl v1::Host for InstanceState {
    async fn open(&mut self, database: String) -> Result<u32, v1::Error> {
        let result = <Self as v2::HostConnection>::open(self, database).await;
        result.map_err(to_legacy_error).map(|s| s.rep())
    }

    async fn execute(
        &mut self,
        connection: u32,
        query: String,
        parameters: Vec<spin_world::v1::sqlite::Value>,
    ) -> Result<spin_world::v1::sqlite::QueryResult, v1::Error> {
        let this = Resource::new_borrow(connection);
        let result = <Self as v2::HostConnection>::execute(
            self,
            this,
            query,
            parameters.into_iter().map(from_legacy_value).collect(),
        )
        .await;
        result.map_err(to_legacy_error).map(to_legacy_query_result)
    }

    async fn close(&mut self, connection: u32) -> anyhow::Result<()> {
        <Self as v2::HostConnection>::drop(self, Resource::new_own(connection))
    }

    fn convert_error(&mut self, error: v1::Error) -> anyhow::Result<v1::Error> {
        Ok(error)
    }
}

fn to_legacy_error(error: v2::Error) -> v1::Error {
    match error {
        v2::Error::NoSuchDatabase => v1::Error::NoSuchDatabase,
        v2::Error::AccessDenied => v1::Error::AccessDenied,
        v2::Error::InvalidConnection => v1::Error::InvalidConnection,
        v2::Error::DatabaseFull => v1::Error::DatabaseFull,
        v2::Error::Io(s) => v1::Error::Io(s),
    }
}

fn to_legacy_query_result(result: v2::QueryResult) -> v1::QueryResult {
    v1::QueryResult {
        columns: result.columns,
        rows: result.rows.into_iter().map(to_legacy_row_result).collect(),
    }
}

fn to_legacy_row_result(result: v2::RowResult) -> v1::RowResult {
    v1::RowResult {
        values: result.values.into_iter().map(to_legacy_value).collect(),
    }
}

fn to_legacy_value(value: v2::Value) -> v1::Value {
    match value {
        v2::Value::Integer(i) => v1::Value::Integer(i),
        v2::Value::Real(r) => v1::Value::Real(r),
        v2::Value::Text(t) => v1::Value::Text(t),
        v2::Value::Blob(b) => v1::Value::Blob(b),
        v2::Value::Null => v1::Value::Null,
    }
}

fn from_legacy_value(value: v1::Value) -> v2::Value {
    match value {
        v1::Value::Integer(i) => v2::Value::Integer(i),
        v1::Value::Real(r) => v2::Value::Real(r),
        v1::Value::Text(t) => v2::Value::Text(t),
        v1::Value::Blob(b) => v2::Value::Blob(b),
        v1::Value::Null => v2::Value::Null,
    }
}

/// A store of connections for all accessible databases for an application
#[async_trait]
pub trait ConnectionsStore: Send + Sync {
    /// Get a `Connection` for a specific database
    async fn get_connection(
        &self,
        database: &str,
    ) -> Result<Option<Arc<dyn Connection + 'static>>, v2::Error>;

    fn has_connection_for(&self, database: &str) -> bool;
}

/// A trait abstracting over operations to a SQLite database
#[async_trait]
pub trait Connection: Send + Sync {
    async fn query(
        &self,
        query: &str,
        parameters: Vec<v2::Value>,
    ) -> Result<v2::QueryResult, v2::Error>;

    async fn execute_batch(&self, statements: &str) -> anyhow::Result<()>;
}
