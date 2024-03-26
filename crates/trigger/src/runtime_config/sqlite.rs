use std::{collections::HashMap, path::PathBuf, sync::Arc};

use crate::{runtime_config::RuntimeConfig, TriggerHooks};
use anyhow::Context;
use spin_common::ui::quoted_path;
use spin_sqlite::{Connection, ConnectionsStore, SqliteComponent, DATABASES_KEY};

use super::RuntimeConfigOpts;

const DEFAULT_SQLITE_DB_FILENAME: &str = "sqlite_db.db";

pub(crate) async fn build_component(
    runtime_config: &RuntimeConfig,
    sqlite_statements: &[String],
) -> anyhow::Result<SqliteComponent> {
    let databases: HashMap<_, _> = runtime_config
        .sqlite_databases()
        .await
        .context("Failed to build sqlite component")?
        .into_iter()
        .collect();
    execute_statements(sqlite_statements, &databases).await?;
    let connections_store =
        Arc::new(SimpleConnectionsStore(databases)) as Arc<dyn ConnectionsStore>;
    Ok(SqliteComponent::new(move |_| connections_store.clone()))
}

/// A `ConnectionStore` based on a `HashMap`
struct SimpleConnectionsStore(HashMap<String, Arc<dyn Connection>>);

#[async_trait::async_trait]
impl ConnectionsStore for SimpleConnectionsStore {
    async fn get_connection(
        &self,
        database: &str,
    ) -> Result<Option<Arc<(dyn Connection + 'static)>>, spin_world::v2::sqlite::Error> {
        Ok(self.0.get(database).cloned())
    }

    fn has_connection_for(&self, database: &str) -> bool {
        self.0.contains_key(database)
    }
}

async fn execute_statements(
    statements: &[String],
    databases: &HashMap<String, Arc<dyn spin_sqlite::Connection>>,
) -> anyhow::Result<()> {
    if statements.is_empty() {
        return Ok(());
    }
    let Some(default) = databases.get("default") else {
        debug_assert!(
            false,
            "the 'default' sqlite database should always be available but for some reason was not"
        );
        return Ok(());
    };

    for m in statements {
        if let Some(file) = m.strip_prefix('@') {
            let sql = std::fs::read_to_string(file).with_context(|| {
                format!("could not read file '{file}' containing sql statements")
            })?;
            default
                .execute_batch(&sql)
                .await
                .with_context(|| format!("failed to execute sql from file '{file}'"))?;
        } else {
            default
                .query(m, Vec::new())
                .await
                .with_context(|| format!("failed to execute statement: '{m}'"))?;
        }
    }
    Ok(())
}

// Holds deserialized options from a `[sqlite_database.<name>]` runtime config section.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum SqliteDatabaseOpts {
    Spin(SpinSqliteDatabaseOpts),
    Libsql(LibsqlOpts),
}

impl SqliteDatabaseOpts {
    pub fn default(runtime_config: &RuntimeConfig) -> Self {
        Self::Spin(SpinSqliteDatabaseOpts::default(runtime_config))
    }

    pub async fn build(
        &self,
        config_opts: &RuntimeConfigOpts,
    ) -> anyhow::Result<Arc<dyn Connection>> {
        match self {
            Self::Spin(opts) => opts.build(config_opts),
            Self::Libsql(opts) => opts.build().await,
        }
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpinSqliteDatabaseOpts {
    pub path: Option<PathBuf>,
}

impl SpinSqliteDatabaseOpts {
    pub fn default(runtime_config: &RuntimeConfig) -> Self {
        let path = runtime_config
            .state_dir()
            .map(|dir| dir.join(DEFAULT_SQLITE_DB_FILENAME));
        Self { path }
    }

    fn build(&self, config_opts: &RuntimeConfigOpts) -> anyhow::Result<Arc<dyn Connection>> {
        use spin_sqlite_inproc::{InProcConnection, InProcDatabaseLocation};

        let location = match self.path.as_ref() {
            Some(path) => {
                let path = super::resolve_config_path(path, config_opts)?;
                // Create the store's parent directory if necessary
                std::fs::create_dir_all(path.parent().unwrap())
                    .context("Failed to create sqlite database directory")?;
                InProcDatabaseLocation::Path(path)
            }
            None => InProcDatabaseLocation::InMemory,
        };
        Ok(Arc::new(InProcConnection::new(location)?))
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibsqlOpts {
    url: String,
    token: String,
}

impl LibsqlOpts {
    async fn build(&self) -> anyhow::Result<Arc<dyn Connection>> {
        let url = check_url(&self.url)
            .with_context(|| {
                format!(
                    "unexpected libSQL URL '{}' in runtime config file ",
                    self.url
                )
            })?
            .to_owned();
        let client = spin_sqlite_libsql::LibsqlClient::create(url, self.token.clone())
            .await
            .context("failed to create SQLite client")?;
        Ok(Arc::new(client))
    }
}

// Checks an incoming url is in the shape we expect
fn check_url(url: &str) -> anyhow::Result<&str> {
    if url.starts_with("https://") || url.starts_with("http://") {
        Ok(url)
    } else {
        Err(anyhow::anyhow!(
            "URL does not start with 'https://' or 'http://'. Spin currently only supports talking to libSQL databases over HTTP(S)"
        ))
    }
}

pub struct SqlitePersistenceMessageHook;

impl TriggerHooks for SqlitePersistenceMessageHook {
    fn app_loaded(
        &mut self,
        app: &spin_app::App,
        runtime_config: &RuntimeConfig,
        _resolver: &Arc<spin_expressions::PreparedResolver>,
    ) -> anyhow::Result<()> {
        if app.components().all(|c| {
            c.get_metadata(DATABASES_KEY)
                .unwrap_or_default()
                .unwrap_or_default()
                .is_empty()
        }) {
            return Ok(());
        }

        match runtime_config.default_sqlite_opts() {
            SqliteDatabaseOpts::Spin(s) => {
                if let Some(path) = &s.path {
                    println!("Storing default SQLite data to {}", quoted_path(path));
                } else {
                    println!("Using in-memory default SQLite database; data will not be saved!");
                }
            }
            SqliteDatabaseOpts::Libsql(l) => {
                println!(
                    "Storing default SQLite data to a libsql database at {}",
                    l.url
                );
            }
        }
        Ok(())
    }
}
