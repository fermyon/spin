mod host;
pub mod runtime_config;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use host::InstanceState;

use async_trait::async_trait;
use spin_factors::{anyhow, Factor};
use spin_locked_app::MetadataKey;
use spin_world::v1::sqlite as v1;
use spin_world::v2::sqlite as v2;

pub use runtime_config::RuntimeConfig;

pub struct SqliteFactor {
    default_label_resolver: Arc<dyn DefaultLabelResolver>,
}

impl SqliteFactor {
    /// Create a new `SqliteFactor`
    ///
    /// Takes a `default_label_resolver` for how to handle when a database label doesn't
    /// have a corresponding runtime configuration.
    pub fn new(default_label_resolver: impl DefaultLabelResolver + 'static) -> Self {
        Self {
            default_label_resolver: Arc::new(default_label_resolver),
        }
    }
}

impl Factor for SqliteFactor {
    type RuntimeConfig = RuntimeConfig;
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
        let connection_creators = ctx
            .take_runtime_config()
            .map(|r| r.connection_creators)
            .unwrap_or_default();

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
        let resolver = self.default_label_resolver.clone();
        let get_connection_creator: host::ConnectionCreatorGetter = Arc::new(move |label| {
            connection_creators
                .get(label)
                .cloned()
                .or_else(|| resolver.default(label))
        });

        ensure_allowed_databases_are_configured(&allowed_databases, |label| {
            get_connection_creator(label).is_some()
        })?;

        Ok(AppState::new(allowed_databases, get_connection_creator))
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
        let get_connection_creator = ctx.app_state().get_connection_creator.clone();
        Ok(InstanceState::new(
            allowed_databases,
            get_connection_creator,
        ))
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

/// Resolves a label to a default connection creator.
pub trait DefaultLabelResolver: Send + Sync {
    /// If there is no runtime configuration for a given database label, return a default connection creator.
    ///
    /// If `Option::None` is returned, the database is not allowed.
    fn default(&self, label: &str) -> Option<Arc<dyn ConnectionCreator>>;
}

#[derive(Clone)]
pub struct AppState {
    /// A map from component id to a set of allowed database labels.
    allowed_databases: HashMap<String, Arc<HashSet<String>>>,
    /// A function for mapping from database name to a connection creator.
    get_connection_creator: host::ConnectionCreatorGetter,
}

impl AppState {
    /// Create a new `AppState`
    pub fn new(
        allowed_databases: HashMap<String, Arc<HashSet<String>>>,
        get_connection_creator: host::ConnectionCreatorGetter,
    ) -> Self {
        Self {
            allowed_databases,
            get_connection_creator,
        }
    }

    /// Get a connection for a given database label.
    ///
    /// Returns `None` if there is no connection creator for the given label.
    pub fn get_connection(&self, label: &str) -> Option<Result<Box<dyn Connection>, v2::Error>> {
        let connection = (self.get_connection_creator)(label)?.create_connection();
        Some(connection)
    }

    /// Returns true if the given database label is used by any component.
    pub fn database_is_used(&self, label: &str) -> bool {
        self.allowed_databases
            .values()
            .any(|stores| stores.contains(label))
    }
}

/// A creator of a connections for a particular SQLite database.
pub trait ConnectionCreator: Send + Sync {
    /// Get a *new* [`Connection`]
    ///
    /// The connection should be a new connection, not a reused one.
    fn create_connection(&self) -> Result<Box<dyn Connection + 'static>, v2::Error>;
}

impl<F> ConnectionCreator for F
where
    F: Fn() -> anyhow::Result<Box<dyn Connection + 'static>> + Send + Sync + 'static,
{
    fn create_connection(&self) -> Result<Box<dyn Connection + 'static>, v2::Error> {
        (self)().map_err(|_| v2::Error::InvalidConnection)
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

    /// A human-readable summary of the connection's configuration
    ///
    /// Example: "libSQL at libsql://example.com"
    fn summary(&self) -> Option<String> {
        None
    }
}
