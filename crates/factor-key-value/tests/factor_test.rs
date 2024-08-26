use anyhow::Context as _;
use spin_factor_key_value::{
    runtime_config::spin::{MakeKeyValueStore, RuntimeConfigResolver},
    KeyValueFactor, RuntimeConfig,
};
use spin_factor_key_value_redis::RedisKeyValueStore;
use spin_factor_key_value_spin::{SpinKeyValueRuntimeConfig, SpinKeyValueStore};
use spin_factors::{FactorRuntimeConfigSource, RuntimeConfigSourceFinalizer, RuntimeFactors};
use spin_factors_test::{toml, TestEnvironment};
use spin_world::v2::key_value::HostStore;
use std::{collections::HashSet, sync::Arc};

#[derive(RuntimeFactors)]
struct TestFactors {
    key_value: KeyValueFactor,
}

#[tokio::test]
async fn default_key_value_works() -> anyhow::Result<()> {
    let mut test_resolver = RuntimeConfigResolver::new();
    test_resolver.register_store_type(SpinKeyValueStore::new(None))?;
    test_resolver
        .add_default_store::<SpinKeyValueStore>("default", SpinKeyValueRuntimeConfig::new(None))?;
    let factors = TestFactors {
        key_value: KeyValueFactor::new(test_resolver),
    };
    let env = TestEnvironment::new(factors).extend_manifest(toml! {
        [component.test-component]
        source = "does-not-exist.wasm"
        key_value_stores = ["default"]
    });
    let state = env.build_instance_state().await?;

    assert_eq!(
        state.key_value.allowed_stores(),
        &["default".into()].into_iter().collect::<HashSet<_>>()
    );
    Ok(())
}

async fn run_test_with_config_and_stores_for_label(
    runtime_config: Option<toml::Table>,
    store_types: Vec<impl MakeKeyValueStore>,
    labels: Vec<&str>,
) -> anyhow::Result<TestFactorsInstanceState> {
    let mut test_resolver = RuntimeConfigResolver::new();
    for store_type in store_types {
        test_resolver.register_store_type(store_type)?;
    }
    let test_resolver = Arc::new(test_resolver);
    let factors = TestFactors {
        key_value: KeyValueFactor::new(test_resolver.clone()),
    };
    let labels_clone = labels.clone();
    let env = TestEnvironment::new(factors)
        .extend_manifest(toml! {
            [component.test-component]
            source = "does-not-exist.wasm"
            key_value_stores = labels_clone
        })
        .runtime_config(TomlConfig::new(test_resolver, runtime_config))?;
    let state = env.build_instance_state().await?;
    assert_eq!(
        labels,
        state.key_value.allowed_stores().iter().collect::<Vec<_>>()
    );

    Ok(state)
}

#[tokio::test]
async fn overridden_default_key_value_works() -> anyhow::Result<()> {
    let runtime_config = toml::toml! {
        [key_value_store.default]
        type = "redis"
        url = "redis://localhost:6379"
    };
    run_test_with_config_and_stores_for_label(
        Some(runtime_config),
        vec![RedisKeyValueStore::new()],
        vec!["default"],
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn custom_spin_key_value_works() -> anyhow::Result<()> {
    let runtime_config = toml::toml! {
        [key_value_store.custom]
        type = "spin"
    };
    run_test_with_config_and_stores_for_label(
        Some(runtime_config),
        vec![SpinKeyValueStore::new(None)],
        vec!["custom"],
    )
    .await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn custom_spin_key_value_works_with_absolute_path() -> anyhow::Result<()> {
    let tmp_dir = tempfile::TempDir::with_prefix("example")?;
    let db_path = tmp_dir.path().join("foo/custom.db");
    // Check that the db does not exist yet - it will exist by the end of the test
    assert!(!db_path.exists());

    let path_str = db_path.to_str().unwrap();
    let runtime_config = toml::toml! {
        [key_value_store.custom]
        type = "spin"
        path = path_str
    };
    let mut state = run_test_with_config_and_stores_for_label(
        Some(runtime_config),
        vec![SpinKeyValueStore::new(Some(
            std::env::current_dir().context("failed to get current directory")?,
        ))],
        vec!["custom"],
    )
    .await?;

    // Actually et a key since store creation is lazy
    let store = state.key_value.open("custom".to_owned()).await??;
    let _ = state.key_value.get(store, "foo".to_owned()).await??;

    // Check that the parent has been created
    assert!(db_path.exists());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn custom_spin_key_value_works_with_relative_path() -> anyhow::Result<()> {
    let tmp_dir = tempfile::TempDir::with_prefix("example")?;
    let db_path = tmp_dir.path().join("custom.db");
    // Check that the db does not exist yet - it will exist by the end of the test
    assert!(!db_path.exists());

    let runtime_config = toml::toml! {
        [key_value_store.custom]
        type = "spin"
        path = "custom.db"
    };
    let mut state = run_test_with_config_and_stores_for_label(
        Some(runtime_config),
        vec![SpinKeyValueStore::new(Some(tmp_dir.path().to_owned()))],
        vec!["custom"],
    )
    .await?;

    // Actually et a key since store creation is lazy
    let store = state.key_value.open("custom".to_owned()).await??;
    let _ = state.key_value.get(store, "foo".to_owned()).await??;

    // Check that the correct store in the config was chosen by verifying the existence of the DB
    assert!(db_path.exists());
    Ok(())
}

#[tokio::test]
async fn custom_redis_key_value_works() -> anyhow::Result<()> {
    let runtime_config = toml::toml! {
        [key_value_store.custom]
        type = "redis"
        url = "redis://localhost:6379"
    };
    run_test_with_config_and_stores_for_label(
        Some(runtime_config),
        vec![RedisKeyValueStore::new()],
        vec!["custom"],
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn misconfigured_spin_key_value_fails() -> anyhow::Result<()> {
    let tmp_dir = tempfile::TempDir::with_prefix("example")?;
    let runtime_config = toml::toml! {
        [key_value_store.custom]
        type = "spin"
        path = "/$$&/bad/path/foo.db"
    };
    let result = run_test_with_config_and_stores_for_label(
        Some(runtime_config),
        vec![SpinKeyValueStore::new(Some(tmp_dir.path().to_owned()))],
        vec!["custom"],
    )
    .await;
    // TODO(rylev): This only fails on my machine due to a read-only file system error.
    // We should consider adding a check for the error message.
    assert!(result.is_err());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
// TODO(rylev): consider removing this test as it is really only a consequence of
// toml deserialization and not a feature of the key-value store.
async fn multiple_custom_key_value_uses_second_store() -> anyhow::Result<()> {
    let tmp_dir = tempfile::TempDir::with_prefix("example")?;
    let db_path = tmp_dir.path().join("custom.db");
    // Check that the db does not exist yet - it will exist by the end of the test
    assert!(!db_path.exists());

    let mut test_resolver = RuntimeConfigResolver::new();
    test_resolver.register_store_type(RedisKeyValueStore::new())?;
    test_resolver.register_store_type(SpinKeyValueStore::new(Some(tmp_dir.path().to_owned())))?;
    let test_resolver = Arc::new(test_resolver);
    let factors = TestFactors {
        key_value: KeyValueFactor::new(test_resolver.clone()),
    };
    let runtime_config = toml::toml! {
        [key_value_store.custom]
        type = "redis"
        url = "redis://localhost:6379"

        [key_value_store.custom]
        type = "spin"
        path = "custom.db"

    };
    let env = TestEnvironment::new(factors)
        .extend_manifest(toml! {
            [component.test-component]
            source = "does-not-exist.wasm"
            key_value_stores = ["custom"]
        })
        .runtime_config(TomlConfig::new(test_resolver, Some(runtime_config)))?;
    let mut state = env.build_instance_state().await?;

    // Actually et a key since store creation is lazy
    let store = state.key_value.open("custom".to_owned()).await??;
    let _ = state.key_value.get(store, "foo".to_owned()).await??;

    assert_eq!(
        state.key_value.allowed_stores(),
        &["custom".into()].into_iter().collect::<HashSet<_>>()
    );
    // Check that the correct store in the config was chosen by verifying the existence of the DB
    assert!(db_path.exists());
    Ok(())
}

struct TomlConfig {
    resolver: Arc<RuntimeConfigResolver>,
    toml: Option<toml::Table>,
}

impl TomlConfig {
    fn new(resolver: Arc<RuntimeConfigResolver>, toml: Option<toml::Table>) -> Self {
        Self { resolver, toml }
    }
}

impl TryFrom<TomlConfig> for TestFactorsRuntimeConfig {
    type Error = anyhow::Error;

    fn try_from(value: TomlConfig) -> Result<Self, Self::Error> {
        Self::from_source(value)
    }
}

impl FactorRuntimeConfigSource<KeyValueFactor> for TomlConfig {
    fn get_runtime_config(&mut self) -> anyhow::Result<Option<RuntimeConfig>> {
        self.resolver.resolve_from_toml(self.toml.as_ref())
    }
}

impl RuntimeConfigSourceFinalizer for TomlConfig {
    fn finalize(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}
