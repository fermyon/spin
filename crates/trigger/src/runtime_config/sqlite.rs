use std::{collections::HashMap, path::PathBuf, sync::Arc};

use crate::runtime_config::RuntimeConfig;
use anyhow::Context;
use spin_sqlite::{DatabaseLocation, SqliteComponent, SqliteConnection};

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
            let c = c.lock().unwrap();
            for m in statements {
                c.execute(m, [])
                    .with_context(|| format!("failed to execute statement: '{m}'"))?;
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
        let location = match self.path.as_ref() {
            Some(path) => {
                let path = super::resolve_config_path(path, config_opts)?;
                // Create the store's parent directory if necessary
                std::fs::create_dir_all(path.parent().unwrap())
                    .context("Failed to create sqlite database directory")?;
                DatabaseLocation::Path(path.join(format!("{name}.db")))
            }
            None => DatabaseLocation::InMemory,
        };
        Ok(Arc::new(SqliteConnection::new(location)))
    }
}
