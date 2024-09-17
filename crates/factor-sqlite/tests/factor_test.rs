use std::{collections::HashSet, sync::Arc};

use spin_factor_sqlite::SqliteFactor;
use spin_factors::{
    anyhow::{self, bail, Context},
    runtime_config::toml::TomlKeyTracker,
    Factor, FactorRuntimeConfigSource, RuntimeConfigSourceFinalizer, RuntimeFactors,
};
use spin_factors_test::{toml, TestEnvironment};
use spin_sqlite::RuntimeConfigResolver;
use spin_world::async_trait;

#[derive(RuntimeFactors)]
struct TestFactors {
    sqlite: SqliteFactor,
}

#[tokio::test]
async fn sqlite_works() -> anyhow::Result<()> {
    let factors = TestFactors {
        sqlite: SqliteFactor::new(),
    };
    let env = TestEnvironment::new(factors).extend_manifest(toml! {
        [component.test-component]
        source = "does-not-exist.wasm"
        sqlite_databases = ["default"]
    });
    let state = env.build_instance_state().await?;

    assert_eq!(
        state.sqlite.allowed_databases(),
        &["default".into()].into_iter().collect::<HashSet<_>>()
    );

    Ok(())
}

#[tokio::test]
async fn errors_when_non_configured_database_used() -> anyhow::Result<()> {
    let factors = TestFactors {
        sqlite: SqliteFactor::new(),
    };
    let env = TestEnvironment::new(factors).extend_manifest(toml! {
        [component.test-component]
        source = "does-not-exist.wasm"
        sqlite_databases = ["foo"]
    });
    let Err(err) = env.build_instance_state().await else {
        bail!("Expected build_instance_state to error but it did not");
    };

    assert!(err
        .to_string()
        .contains("One or more components use SQLite databases which are not defined."));

    Ok(())
}

#[tokio::test]
async fn no_error_when_database_is_configured() -> anyhow::Result<()> {
    let factors = TestFactors {
        sqlite: SqliteFactor::new(),
    };
    let runtime_config = toml! {
        [sqlite_database.foo]
        type = "spin"
    };
    let sqlite_config = RuntimeConfigResolver::new(None, "/".into());
    let env = TestEnvironment::new(factors)
        .extend_manifest(toml! {
            [component.test-component]
            source = "does-not-exist.wasm"
            sqlite_databases = ["foo"]
        })
        .runtime_config(TomlRuntimeSource::new(&runtime_config, sqlite_config))?;
    env.build_instance_state()
        .await
        .context("build_instance_state failed")?;
    Ok(())
}

struct TomlRuntimeSource<'a> {
    table: TomlKeyTracker<'a>,
    runtime_config_resolver: RuntimeConfigResolver,
}

impl<'a> TomlRuntimeSource<'a> {
    fn new(table: &'a toml::Table, runtime_config_resolver: RuntimeConfigResolver) -> Self {
        Self {
            table: TomlKeyTracker::new(table),
            runtime_config_resolver,
        }
    }
}

impl FactorRuntimeConfigSource<SqliteFactor> for TomlRuntimeSource<'_> {
    fn get_runtime_config(
        &mut self,
    ) -> anyhow::Result<Option<<SqliteFactor as Factor>::RuntimeConfig>> {
        self.runtime_config_resolver.resolve_from_toml(&self.table)
    }
}

impl RuntimeConfigSourceFinalizer for TomlRuntimeSource<'_> {
    fn finalize(&mut self) -> anyhow::Result<()> {
        self.table.validate_all_keys_used()?;
        Ok(())
    }
}

impl TryFrom<TomlRuntimeSource<'_>> for TestFactorsRuntimeConfig {
    type Error = anyhow::Error;

    fn try_from(value: TomlRuntimeSource<'_>) -> Result<Self, Self::Error> {
        Self::from_source(value)
    }
}

/// A connection creator that always returns an error.
struct InvalidConnectionCreator;

#[async_trait]
impl spin_factor_sqlite::ConnectionCreator for InvalidConnectionCreator {
    async fn create_connection(
        &self,
        label: &str,
    ) -> Result<Box<dyn spin_factor_sqlite::Connection + 'static>, spin_world::v2::sqlite::Error>
    {
        let _ = label;
        Err(spin_world::v2::sqlite::Error::InvalidConnection)
    }
}
