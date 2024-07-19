mod host;
pub mod runtime_config;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use host::InstanceState;

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use spin_factors::{anyhow, Factor};
use spin_locked_app::MetadataKey;
use spin_world::v1::sqlite as v1;
use spin_world::v2::sqlite as v2;

pub struct SqliteFactor<C> {
    runtime_config_resolver: Arc<dyn runtime_config::RuntimeConfigResolver<C>>,
}

impl<C> SqliteFactor<C> {
    /// Create a new `SqliteFactor`
    pub fn new(
        runtime_config_resolver: impl runtime_config::RuntimeConfigResolver<C> + 'static,
    ) -> Self {
        Self {
            runtime_config_resolver: Arc::new(runtime_config_resolver),
        }
    }
}

impl<C: DeserializeOwned + 'static> Factor for SqliteFactor<C> {
    type RuntimeConfig = runtime_config::RuntimeConfig<C>;
    type AppState = AppState;
    type InstanceBuilder = InstanceState;

    fn init<T: Send + 'static>(
        &mut self,
        mut ctx: spin_factors::InitContext<T, Self>,
    ) -> anyhow::Result<()> {
        ctx.link_bindings(v1::add_to_linker)?;
        ctx.link_bindings(v2::add_to_linker)?;
        Ok(())
    }

    fn configure_app<T: spin_factors::RuntimeFactors>(
        &self,
        mut ctx: spin_factors::ConfigureAppContext<T, Self>,
    ) -> anyhow::Result<Self::AppState> {
        let mut connection_pools = HashMap::new();
        if let Some(runtime_config) = ctx.take_runtime_config() {
            for (database_label, config) in runtime_config.store_configs {
                let pool = self.runtime_config_resolver.get_pool(config)?;
                connection_pools.insert(database_label, pool);
            }
        }

        let allowed_databases = ctx
            .app()
            .components()
            .map(|component| {
                Ok((
                    component.id().to_string(),
                    Arc::new(
                        component
                            .get_metadata(ALLOWED_DATABASES_KEY)?
                            .unwrap_or_default()
                            .into_iter()
                            .collect::<HashSet<_>>(),
                    ),
                ))
            })
            .collect::<anyhow::Result<HashMap<_, _>>>()?;
        let resolver = self.runtime_config_resolver.clone();
        let get_connection_pool: host::ConnectionPoolGetter = Arc::new(move |label| {
            connection_pools
                .get(label)
                .cloned()
                .or_else(|| resolver.default(label))
        });

        ensure_allowed_databases_are_configured(&allowed_databases, |label| {
            get_connection_pool(label).is_some()
        })?;

        Ok(AppState {
            allowed_databases,
            get_connection_pool,
        })
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
        let get_connection_pool = ctx.app_state().get_connection_pool.clone();
        Ok(InstanceState::new(allowed_databases, get_connection_pool))
    }
}

/// Ensure that all the databases in the allowed databases list for each component are configured
fn ensure_allowed_databases_are_configured(
    allowed_databases: &HashMap<String, Arc<HashSet<String>>>,
    is_configured: impl Fn(&str) -> bool,
) -> anyhow::Result<()> {
    let mut errors = Vec::new();
    for (component_id, allowed_dbs) in allowed_databases {
        for allowed in allowed_dbs.iter() {
            if !is_configured(allowed) {
                errors.push(format!(
                    "- Component {component_id} uses database '{allowed}'"
                ));
            }
        }
    }

    if !errors.is_empty() {
        let prologue = vec![
            "One or more components use SQLite databases which are not defined.",
            "Check the spelling, or pass a runtime configuration file that defines these stores.",
            "See https://developer.fermyon.com/spin/dynamic-configuration#sqlite-storage-runtime-configuration",
            "Details:",
        ];
        let lines: Vec<_> = prologue
            .into_iter()
            .map(|s| s.to_owned())
            .chain(errors)
            .collect();
        return Err(anyhow::anyhow!(lines.join("\n")));
    }
    Ok(())
}

pub const ALLOWED_DATABASES_KEY: MetadataKey<Vec<String>> = MetadataKey::new("databases");

pub struct AppState {
    /// A map from component id to a set of allowed database labels.
    allowed_databases: HashMap<String, Arc<HashSet<String>>>,
    /// A function for mapping from database name to a connection pool
    get_connection_pool: host::ConnectionPoolGetter,
}

/// A pool of connections for a particular SQLite database
#[async_trait]
pub trait ConnectionPool: Send + Sync {
    /// Get a `Connection` from the pool
    async fn get_connection(&self) -> Result<Arc<dyn Connection + 'static>, v2::Error>;
}

/// A simple [`ConnectionPool`] that always creates a new connection.
pub struct SimpleConnectionPool(
    Box<dyn Fn() -> anyhow::Result<Arc<dyn Connection + 'static>> + Send + Sync>,
);

impl SimpleConnectionPool {
    /// Create a new `SimpleConnectionPool` with the given connection factory.
    pub fn new(
        factory: impl Fn() -> anyhow::Result<Arc<dyn Connection + 'static>> + Send + Sync + 'static,
    ) -> Self {
        Self(Box::new(factory))
    }
}

#[async_trait::async_trait]
impl ConnectionPool for SimpleConnectionPool {
    async fn get_connection(&self) -> Result<Arc<dyn Connection + 'static>, v2::Error> {
        (self.0)().map_err(|_| v2::Error::InvalidConnection)
    }
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
