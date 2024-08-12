//! Spin's default handling of the runtime configuration for SQLite databases.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use serde::Deserialize;
use spin_factors::{
    anyhow::{self, Context as _},
    runtime_config::toml::GetTomlValue,
};
use spin_world::v2::sqlite as v2;
use tokio::sync::OnceCell;

use crate::{Connection, ConnectionCreator, DefaultLabelResolver, FunctionConnectionCreator};

/// Spin's default handling of the runtime configuration for SQLite databases.
///
/// This type implements the [`RuntimeConfigResolver`] trait and provides a way to
/// opt into the default behavior of Spin's SQLite database handling.
pub struct SpinSqliteRuntimeConfig {
    default_database_dir: PathBuf,
    local_database_dir: PathBuf,
}

impl SpinSqliteRuntimeConfig {
    /// Create a new `SpinSqliteRuntimeConfig`
    ///
    /// This takes as arguments:
    /// * the directory to use as the default location for SQLite databases. Usually this
    /// will be the path to the `.spin` state directory.
    /// * the *absolute* path to the directory from which relative paths to local SQLite
    /// databases are resolved.  (this should most likely be the path to the runtime-config
    /// file or the current working dir).
    ///
    /// Panics if either `default_database_dir` or `local_database_dir` are not absolute paths.
    pub fn new(default_database_dir: PathBuf, local_database_dir: PathBuf) -> Self {
        assert!(
            default_database_dir.is_absolute(),
            "default_database_dir must be an absolute path"
        );
        assert!(
            local_database_dir.is_absolute(),
            "local_database_dir must be an absolute path"
        );
        Self {
            default_database_dir,
            local_database_dir,
        }
    }

    /// Get the runtime configuration for SQLite databases from a TOML table.
    ///
    /// Expects table to be in the format:
    /// ````toml
    /// [sqlite_database.$database-label]
    /// type = "$database-type"
    /// ... extra type specific configuration ...
    /// ```
    pub fn config_from_table<T: GetTomlValue + AsRef<toml::Table>>(
        &self,
        table: &T,
    ) -> anyhow::Result<Option<super::RuntimeConfig>> {
        let Some(table) = table.get("sqlite_database") else {
            return Ok(None);
        };
        let config: std::collections::HashMap<String, RuntimeConfig> = table.clone().try_into()?;
        let pools = config
            .into_iter()
            .map(|(k, v)| Ok((k, self.get_pool(v)?)))
            .collect::<anyhow::Result<_>>()?;
        Ok(Some(super::RuntimeConfig {
            connection_creators: pools,
        }))
    }

    /// Get a connection pool for a given runtime configuration.
    pub fn get_pool(&self, config: RuntimeConfig) -> anyhow::Result<Arc<dyn ConnectionCreator>> {
        let database_kind = config.type_.as_str();
        let pool = match database_kind {
            "spin" => {
                let config: LocalDatabase = config.config.try_into()?;
                config.pool(&self.local_database_dir)?
            }
            "libsql" => {
                let config: LibSqlDatabase = config.config.try_into()?;
                config.pool()?
            }
            _ => anyhow::bail!("Unknown database kind: {database_kind}"),
        };
        Ok(Arc::new(pool))
    }
}

#[derive(Deserialize)]
pub struct RuntimeConfig {
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(flatten)]
    pub config: toml::Table,
}

impl DefaultLabelResolver for SpinSqliteRuntimeConfig {
    fn default(&self, label: &str) -> Option<Arc<dyn ConnectionCreator>> {
        // Only default the database labeled "default".
        if label != "default" {
            return None;
        }

        let path = self.default_database_dir.join(DEFAULT_SQLITE_DB_FILENAME);
        let factory = move || {
            let location = spin_sqlite_inproc::InProcDatabaseLocation::Path(path.clone());
            let connection = spin_sqlite_inproc::InProcConnection::new(location)?;
            Ok(Box::new(connection) as _)
        };
        let pool = FunctionConnectionCreator::new(factory);
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
    ///
    /// `base_dir` is the base directory path from which `path` is resolved if it is a relative path.
    fn pool(self, base_dir: &Path) -> anyhow::Result<FunctionConnectionCreator> {
        let location = match self.path {
            Some(path) => {
                let path = resolve_relative_path(&path, base_dir);
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
            Ok(Box::new(connection) as _)
        };
        Ok(FunctionConnectionCreator::new(factory))
    }
}

/// Resolve a relative path against a base dir.
///
/// If the path is absolute, it is returned as is. Otherwise, it is resolved against the base dir.
fn resolve_relative_path(path: &Path, base_dir: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_owned();
    }
    base_dir.join(path)
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
    fn pool(self) -> anyhow::Result<FunctionConnectionCreator> {
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
            Ok(Box::new(connection) as _)
        };
        Ok(FunctionConnectionCreator::new(factory))
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
