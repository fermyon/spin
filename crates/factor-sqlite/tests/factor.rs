use std::{collections::HashSet, sync::Arc};

use factor_sqlite::SqliteFactor;
use spin_factors::{anyhow, RuntimeFactors};
use spin_factors_test::{toml, TestEnvironment};

#[derive(RuntimeFactors)]
struct TestFactors {
    sqlite: SqliteFactor,
}

#[tokio::test]
async fn sqlite_works() -> anyhow::Result<()> {
    let test_resolver = RuntimeConfigResolver;
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

struct RuntimeConfigResolver;

impl factor_sqlite::runtime_config::RuntimeConfigResolver for RuntimeConfigResolver {
    fn get_pool(
        &self,
        database_kind: &str,
        config: toml::Table,
    ) -> anyhow::Result<Arc<dyn factor_sqlite::ConnectionPool>> {
        let _ = (database_kind, config);
        todo!()
    }

    fn default(&self, label: &str) -> Option<Arc<dyn factor_sqlite::ConnectionPool>> {
        let _ = label;
        todo!()
    }
}
