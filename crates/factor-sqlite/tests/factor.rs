use std::{collections::HashSet, sync::Arc};

use factor_sqlite::SqliteFactor;
use serde::Deserialize;
use spin_factors::{
    anyhow::{self, bail},
    RuntimeFactors,
};
use spin_factors_test::{toml, TestEnvironment};

#[derive(RuntimeFactors)]
struct TestFactors {
    sqlite: SqliteFactor<RuntimeConfig>,
}

#[tokio::test]
async fn sqlite_works() -> anyhow::Result<()> {
    let test_resolver = RuntimeConfigResolver::new(Some("default"));
    let factors = TestFactors {
        sqlite: SqliteFactor::new(test_resolver),
    };
    let env = TestEnvironment::default_manifest_extend(toml! {
        [component.test-component]
        source = "does-not-exist.wasm"
        sqlite_databases = ["default"]
    });
    let state = env.build_instance_state(factors).await?;

    assert_eq!(
        state.sqlite.allowed_databases(),
        &["default".into()].into_iter().collect::<HashSet<_>>()
    );

    Ok(())
}

#[tokio::test]
async fn errors_when_non_configured_database_used() -> anyhow::Result<()> {
    let test_resolver = RuntimeConfigResolver::new(None);
    let factors = TestFactors {
        sqlite: SqliteFactor::new(test_resolver),
    };
    let env = TestEnvironment::default_manifest_extend(toml! {
        [component.test-component]
        source = "does-not-exist.wasm"
        sqlite_databases = ["foo"]
    });
    let Err(err) = env.build_instance_state(factors).await else {
        bail!("Expected build_instance_state to error but it did not");
    };

    assert!(err
        .to_string()
        .contains("One or more components use SQLite databases which are not defined."));

    Ok(())
}

#[tokio::test]
async fn no_error_when_database_is_configured() -> anyhow::Result<()> {
    let test_resolver = RuntimeConfigResolver::new(None);
    let factors = TestFactors {
        sqlite: SqliteFactor::new(test_resolver),
    };
    let mut env = TestEnvironment::default_manifest_extend(toml! {
        [component.test-component]
        source = "does-not-exist.wasm"
        sqlite_databases = ["foo"]
    });
    env.runtime_config = toml! {
        [sqlite_database.foo]
        type = "sqlite"
    };
    if let Err(e) = env.build_instance_state(factors).await {
        bail!("Expected build_instance_state to succeed but it errored: {e}");
    }

    Ok(())
}

/// Will return an `InvalidConnectionPool` for all runtime configured databases and the supplied default database.
struct RuntimeConfigResolver {
    default: Option<String>,
}

impl RuntimeConfigResolver {
    fn new(default: Option<&str>) -> Self {
        Self {
            default: default.map(Into::into),
        }
    }
}

impl factor_sqlite::runtime_config::RuntimeConfigResolver<RuntimeConfig> for RuntimeConfigResolver {
    fn get_pool(
        &self,
        config: RuntimeConfig,
    ) -> anyhow::Result<Arc<dyn factor_sqlite::ConnectionPool>> {
        let _ = config;
        Ok(Arc::new(InvalidConnectionPool))
    }

    fn default(&self, label: &str) -> Option<Arc<dyn factor_sqlite::ConnectionPool>> {
        let Some(default) = &self.default else {
            return None;
        };
        (default == label).then_some(Arc::new(InvalidConnectionPool))
    }
}

/// A connection pool that always returns an error.
struct InvalidConnectionPool;

#[async_trait::async_trait]
impl factor_sqlite::ConnectionPool for InvalidConnectionPool {
    async fn get_connection(
        &self,
    ) -> Result<Arc<dyn factor_sqlite::Connection + 'static>, spin_world::v2::sqlite::Error> {
        Err(spin_world::v2::sqlite::Error::InvalidConnection)
    }
}

#[derive(Deserialize)]
pub struct RuntimeConfig {
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(flatten)]
    pub config: toml::Table,
}
