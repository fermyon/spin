mod host;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use host::InstanceState;

use async_trait::async_trait;
use serde::Deserialize;
use spin_factors::{anyhow, Factor, FactorRuntimeConfig, RuntimeFactors};
use spin_locked_app::MetadataKey;
use spin_world::v1::sqlite as v1;
use spin_world::v2::sqlite as v2;

pub struct SqliteFactor {
    runtime_config_resolver: Arc<dyn RuntimeConfigResolver + Sync + Send + 'static>,
}

impl SqliteFactor {
    /// Create a new `SqliteFactor`
    pub fn new(
        runtime_config_resolver: impl RuntimeConfigResolver + Send + Sync + 'static,
    ) -> Self {
        Self {
            runtime_config_resolver: Arc::new(runtime_config_resolver),
        }
    }
}

pub const ALLOWED_DATABASES_KEY: MetadataKey<Vec<String>> = MetadataKey::new("databases");

impl Factor for SqliteFactor {
    type RuntimeConfig = RuntimeConfig;
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
        mut ctx: spin_factors::ConfigureAppContext<T, Self>,
    ) -> anyhow::Result<Self::AppState> {
        let mut connection_pools = HashMap::new();
        if let Some(runtime_config) = ctx.take_runtime_config() {
            for (database_label, StoreConfig { type_, config }) in runtime_config.store_configs {
                let pool = self.runtime_config_resolver.get_pool(&type_, config)?;
                connection_pools.insert(database_label, pool);
            }
        }

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
        let resolver = self.runtime_config_resolver.clone();
        Ok(AppState {
            allowed_databases,
            connection_pools: Arc::new(move |label| {
                connection_pools
                    .get(label)
                    .cloned()
                    .or_else(|| resolver.default(label))
            }),
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
        let connection_pools = ctx.app_state().connection_pools.clone();
        Ok(InstanceState::new(allowed_databases, connection_pools))
    }
}

pub struct AppState {
    /// A map from component id to a set of allowed databases
    allowed_databases: HashMap<String, Arc<HashSet<String>>>,
    /// A map from database name to a connection pool
    connection_pools: host::ConnectionPoolGetter,
}

#[derive(Deserialize)]
#[serde(transparent)]
pub struct RuntimeConfig {
    store_configs: HashMap<String, StoreConfig>,
}

impl FactorRuntimeConfig for RuntimeConfig {
    const KEY: &'static str = "sqlite_database";
}

#[derive(Deserialize)]
struct StoreConfig {
    #[serde(rename = "type")]
    type_: String,
    #[serde(flatten)]
    config: toml::Table,
}

/// Resolves some piece of runtime configuration to a connection pool
pub trait RuntimeConfigResolver {
    /// Get a connection pool for a given runtime configuration type and the raw configuration.
    fn get_pool(
        &self,
        r#type: &str,
        config: toml::Table,
    ) -> anyhow::Result<Arc<dyn ConnectionPool>>;

    /// If there is no runtime configuration for a given database label, return a default connection pool.
    ///
    /// If `Option::None` is returned, the database is not allowed.
    fn default(&self, label: &str) -> Option<Arc<dyn ConnectionPool>>;
}

/// A store of connections for all accessible databases for an application
#[async_trait]
pub trait ConnectionPool: Send + Sync {
    /// Get a `Connection` from the pool
    async fn get_connection(&self) -> Result<Arc<dyn Connection + 'static>, v2::Error>;
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
