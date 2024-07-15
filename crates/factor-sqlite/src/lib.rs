mod host;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use host::InstanceState;

use async_trait::async_trait;
use spin_factors::{anyhow, Factor, RuntimeFactors};
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
        Ok(InstanceState::new(
            allowed_databases,
            self.connections_store.clone(),
        ))
    }
}

pub struct AppState {
    allowed_databases: HashMap<String, Arc<HashSet<String>>>,
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
