//! Spin's default handling of the runtime configuration for SQLite databases.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use serde::Deserialize;
use spin_factors::anyhow::{self, Context as _};
use spin_world::v2::sqlite as v2;
use tokio::sync::OnceCell;

use crate::{Connection, ConnectionPool, SimpleConnectionPool};

use super::RuntimeConfigResolver;

/// Spin's default handling of the runtime configuration for SQLite databases.
///
/// This type implements the [`RuntimeConfigResolver`] trait and provides a way to
/// opt into the default behavior of Spin's SQLite database handling.
pub struct SpinSqliteRuntimeConfig {
    state_dir: PathBuf,
    base_path: Option<PathBuf>,
}

impl SpinSqliteRuntimeConfig {
    /// Create a new `SpinSqliteRuntimeConfig`
    ///
    /// This takes as arguments:
    /// * the state directory path (i.e., the path to the `.spin` file). This
    /// is used to derive the default path a local SQLite database file.
    /// * the base path from which relative paths referenced in configuration are resolved
    /// (this should most likely be the path to the runtime-config file). If
    /// `None`, the current working directory is used.
    pub fn new(state_dir: PathBuf, base_path: Option<PathBuf>) -> Self {
        Self {
            state_dir,
            base_path,
        }
    }
}

impl RuntimeConfigResolver for SpinSqliteRuntimeConfig {
    fn get_pool(
        &self,
        database_kind: &str,
        config: toml::Table,
    ) -> anyhow::Result<Arc<dyn ConnectionPool>> {
        let pool = match database_kind {
            "spin" => {
                let config: LocalDatabase = config.try_into()?;
                config.pool(self.base_path.as_deref())?
            }
            "libsql" => {
                let config: LibSqlDatabase = config.try_into()?;
                config.pool()?
            }
            _ => anyhow::bail!("Unknown database kind: {}", database_kind),
        };
        Ok(Arc::new(pool))
    }

    fn default(&self, label: &str) -> Option<Arc<dyn ConnectionPool>> {
        // Only default the database labeled "default".
        if label != "default" {
            return None;
        }

        let path = self.state_dir.join(DEFAULT_SQLITE_DB_FILENAME);
        let factory = move || {
            let location = spin_sqlite_inproc::InProcDatabaseLocation::Path(path.clone());
            let connection = spin_sqlite_inproc::InProcConnection::new(location)?;
            Ok(Arc::new(connection) as _)
        };
        let pool = SimpleConnectionPool::new(factory);
        Some(Arc::new(pool))
    }
}

const DEFAULT_SQLITE_DB_FILENAME: &str = "sqlite_db.db";

#[async_trait::async_trait]
impl Connection for spin_sqlite_inproc::InProcConnection {
    async fn query(
        &self,
        query: &str,
        parameters: Vec<v2::Value>,
    ) -> Result<v2::QueryResult, v2::Error> {
        <Self as spin_sqlite::Connection>::query(self, query, parameters).await
    }

    async fn execute_batch(&self, statements: &str) -> anyhow::Result<()> {
        <Self as spin_sqlite::Connection>::execute_batch(self, statements).await
    }
}

/// A wrapper around a libSQL connection that implements the [`Connection`] trait.
struct LibSqlConnection {
    url: String,
    token: String,
    // Since the libSQL client can only be created asynchronously, we wait until
    // we're in the `Connection` implementation to create. Since we only want to do
    // this once, we use a `OnceCell` to store it.
    inner: OnceCell<spin_sqlite_libsql::LibsqlClient>,
}

impl LibSqlConnection {
    fn new(url: String, token: String) -> Self {
        Self {
            url,
            token,
            inner: OnceCell::new(),
        }
    }

    async fn get_client(&self) -> Result<&spin_sqlite_libsql::LibsqlClient, v2::Error> {
        self.inner
            .get_or_try_init(|| async {
                spin_sqlite_libsql::LibsqlClient::create(self.url.clone(), self.token.clone())
                    .await
                    .context("failed to create SQLite client")
            })
            .await
            .map_err(|_| v2::Error::InvalidConnection)
    }
}

#[async_trait::async_trait]
impl Connection for LibSqlConnection {
    async fn query(
        &self,
        query: &str,
        parameters: Vec<v2::Value>,
    ) -> Result<v2::QueryResult, v2::Error> {
        let client = self.get_client().await?;
        <spin_sqlite_libsql::LibsqlClient as spin_sqlite::Connection>::query(
            client, query, parameters,
        )
        .await
    }

    async fn execute_batch(&self, statements: &str) -> anyhow::Result<()> {
        let client = self.get_client().await?;
        <spin_sqlite_libsql::LibsqlClient as spin_sqlite::Connection>::execute_batch(
            client, statements,
        )
        .await
    }
}

/// Configuration for a local SQLite database.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalDatabase {
    pub path: Option<PathBuf>,
}

impl LocalDatabase {
    /// Create a new connection pool for a local database.
    fn pool(self, base_path: Option<&Path>) -> anyhow::Result<SimpleConnectionPool> {
        let location = match self.path {
            Some(path) => {
                // TODO: `base_path` should be passed in from the runtime config
                let path = resolve_relative_path(&path, base_path)?;
                // Create the store's parent directory if necessary
                // unwrapping the parent is fine, because `resolve_relative_path`` will always return a path with a parent
                std::fs::create_dir_all(path.parent().unwrap())
                    .context("Failed to create sqlite database directory")?;
                spin_sqlite_inproc::InProcDatabaseLocation::Path(path)
            }
            None => spin_sqlite_inproc::InProcDatabaseLocation::InMemory,
        };
        let factory = move || {
            let connection = spin_sqlite_inproc::InProcConnection::new(location.clone())?;
            Ok(Arc::new(connection) as _)
        };
        Ok(SimpleConnectionPool::new(factory))
    }
}

/// Resolve a relative path against an optional base path.
fn resolve_relative_path(path: &Path, base_path: Option<&Path>) -> anyhow::Result<PathBuf> {
    if path.is_absolute() {
        return Ok(path.to_owned());
    }
    let base_path = match base_path {
        Some(base_path) => base_path
            .parent()
            .with_context(|| {
                format!(
                    "failed to get parent of runtime config file path \"{}\"",
                    base_path.display()
                )
            })?
            .to_owned(),
        None => std::env::current_dir().context("failed to get current directory")?,
    };
    Ok(base_path.join(path))
}

/// Configuration for a libSQL database.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibSqlDatabase {
    url: String,
    token: String,
}

impl LibSqlDatabase {
    /// Create a new connection pool for a libSQL database.
    fn pool(self) -> anyhow::Result<SimpleConnectionPool> {
        let url = check_url(&self.url)
            .with_context(|| {
                format!(
                    "unexpected libSQL URL '{}' in runtime config file ",
                    self.url
                )
            })?
            .to_owned();
        let factory = move || {
            let connection = LibSqlConnection::new(url.clone(), self.token.clone());
            Ok(Arc::new(connection) as _)
        };
        Ok(SimpleConnectionPool::new(factory))
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
