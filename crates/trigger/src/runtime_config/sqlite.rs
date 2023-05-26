use std::{collections::HashMap, path::PathBuf, sync::Arc};

use crate::{runtime_config::RuntimeConfig, TriggerHooks};
use anyhow::Context;
use spin_sqlite::{SqliteComponent, DATABASES_KEY};

use super::RuntimeConfigOpts;

pub type SqliteDatabase = Arc<dyn spin_sqlite::ConnectionManager>;

pub(crate) fn build_component(
    runtime_config: &RuntimeConfig,
    sqlite_statements: &[String],
) -> anyhow::Result<SqliteComponent> {
    let databases: HashMap<_, _> = runtime_config
        .sqlite_databases()
        .context("Failed to build sqlite component")?
        .into_iter()
        .collect();
    execute_statements(sqlite_statements, &databases)?;
    Ok(SqliteComponent::new(databases))
}

fn execute_statements(
    statements: &[String],
    databases: &HashMap<String, Arc<dyn spin_sqlite::ConnectionManager>>,
) -> anyhow::Result<()> {
    if !statements.is_empty() {
        if let Some(default) = databases.get("default") {
            let c = default.get_connection().context(
                "could not get connection to default database in order to execute statements",
            )?;
            for m in statements {
                if let Some(file) = m.strip_prefix('@') {
                    let sql = std::fs::read_to_string(file).with_context(|| {
                        format!("could not read file '{file}' containing sql statements")
                    })?;
                    c.execute_batch(&sql)
                        .with_context(|| format!("failed to execute sql from file '{file}'"))?;
                } else {
                    c.query(m, Vec::new())
                        .with_context(|| format!("failed to execute statement: '{m}'"))?;
                }
            }
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

    pub fn build(
        &self,
        name: &str,
        config_opts: &RuntimeConfigOpts,
    ) -> anyhow::Result<SqliteDatabase> {
        match self {
            Self::Spin(opts) => opts.build(name, config_opts),
            Self::Libsql(opts) => opts.build(name, config_opts),
        }
    }
}

#[derive(Clone, Debug, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpinSqliteDatabaseOpts {
    pub path: Option<PathBuf>,
}

impl SpinSqliteDatabaseOpts {
    pub fn default(runtime_config: &RuntimeConfig) -> Self {
        // If the state dir is set, build the default path
        let path = runtime_config.state_dir();
        Self { path }
    }

    fn build(&self, name: &str, config_opts: &RuntimeConfigOpts) -> anyhow::Result<SqliteDatabase> {
        use spin_sqlite_inproc::{InProcConnectionManager, InProcDatabaseLocation};

        let location = match self.path.as_ref() {
            Some(path) => {
                let path = super::resolve_config_path(path, config_opts)?;
                // Create the store's parent directory if necessary
                std::fs::create_dir_all(path.parent().unwrap())
                    .context("Failed to create sqlite database directory")?;
                InProcDatabaseLocation::Path(path.join(format!("{name}.db")))
            }
            None => InProcDatabaseLocation::InMemory,
        };
        Ok(Arc::new(InProcConnectionManager::new(location)))
    }
}

#[derive(Clone, Debug, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibsqlOpts {
    url: String,
    token: String,
}

impl LibsqlOpts {
    fn build(
        &self,
        _name: &str,
        _config_opts: &RuntimeConfigOpts,
    ) -> anyhow::Result<SqliteDatabase> {
        Ok(Arc::new(spin_sqlite_libsql::LibsqlClient::new(
            self.url.clone(),
            self.token.clone(),
        )))
    }
}

pub struct SqlitePersistenceMessageHook;

impl TriggerHooks for SqlitePersistenceMessageHook {
    fn app_loaded(
        &mut self,
        app: &spin_app::App,
        runtime_config: &RuntimeConfig,
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
                    println!("Storing default SQLite data to local SQLite database at {path:?}.");
                } else {
                    println!("Using in-memory default SQLite database.");
                }
            }
            SqliteDatabaseOpts::Libsql(l) => {
                println!(
                    "Storing default SQLite data to a remote LibSQL database at {}",
                    l.url
                );
            }
        }
        Ok(())
    }
}
