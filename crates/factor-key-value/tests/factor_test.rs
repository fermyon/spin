use anyhow::Context;
use spin_factor_key_value::{
    KeyValueFactor, MakeKeyValueStore, RuntimeConfig, RuntimeConfigResolver, StoreConfig,
};
use spin_factor_key_value_redis::RedisKeyValueStore;
use spin_factor_key_value_spin::{SpinKeyValueRuntimeConfig, SpinKeyValueStore};
use spin_factors::{FactorRuntimeConfigSource, RuntimeConfigSourceFinalizer, RuntimeFactors};
use spin_factors_test::{toml, TestEnvironment};
use std::{collections::HashSet, sync::Arc};

#[derive(RuntimeFactors)]
struct TestFactors {
    key_value: KeyValueFactor,
}

fn default_key_value_resolver() -> anyhow::Result<(RuntimeConfigResolver, tempdir::TempDir)> {
    let mut test_resolver = RuntimeConfigResolver::new();
    test_resolver.register_store_type(SpinKeyValueStore::new(
        std::env::current_dir().context("failed to get current directory")?,
    ))?;
    let tmp_dir = tempdir::TempDir::new("example")?;
    let path = tmp_dir.path().to_path_buf();
    let default_config = SpinKeyValueRuntimeConfig::default(Some(path));
    let store_config = StoreConfig::new(
        SpinKeyValueStore::RUNTIME_CONFIG_TYPE.to_string(),
        default_config,
    )?;
    test_resolver.add_default_store("default", store_config);
    Ok((test_resolver, tmp_dir))
}

#[tokio::test]
async fn default_key_value_works() -> anyhow::Result<()> {
    let (test_resolver, dir) = default_key_value_resolver()?;
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
    // Ensure the database directory is created
    assert!(dir.path().exists());
    Ok(())
}

async fn run_test_with_config_and_stores_for_label(
    runtime_config: Option<toml::Table>,
    store_types: Vec<impl MakeKeyValueStore>,
    labels: Vec<&str>,
) -> anyhow::Result<()> {
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

    Ok(())
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
        vec![RedisKeyValueStore],
        vec!["default"],
    )
    .await
}

#[tokio::test]
async fn custom_spin_key_value_works() -> anyhow::Result<()> {
    let runtime_config = toml::toml! {
        [key_value_store.custom]
        type = "spin"
    };
    run_test_with_config_and_stores_for_label(
        Some(runtime_config),
        vec![SpinKeyValueStore::new(
            std::env::current_dir().context("failed to get current directory")?,
        )],
        vec!["custom"],
    )
    .await
}

#[tokio::test]
async fn custom_spin_key_value_works_with_absolute_path() -> anyhow::Result<()> {
    let tmp_dir = tempdir::TempDir::new("example")?;
    let path = tmp_dir.path().join("custom.db");
    let path_str = path.to_str().unwrap();
    let runtime_config = toml::toml! {
        [key_value_store.custom]
        type = "spin"
        path = path_str
    };
    run_test_with_config_and_stores_for_label(
        Some(runtime_config),
        vec![SpinKeyValueStore::new(
            std::env::current_dir().context("failed to get current directory")?,
        )],
        vec!["custom"],
    )
    .await?;
    assert!(tmp_dir.path().exists());
    Ok(())
}

#[tokio::test]
async fn custom_spin_key_value_works_with_relative_path() -> anyhow::Result<()> {
    let tmp_dir = tempdir::TempDir::new("example")?;
    let path = tmp_dir.path().to_owned();
    let runtime_config = toml::toml! {
        [key_value_store.custom]
        type = "spin"
        path = "custom.db"
    };
    run_test_with_config_and_stores_for_label(
        Some(runtime_config),
        vec![SpinKeyValueStore::new(path)],
        vec!["custom"],
    )
    .await?;
    assert!(tmp_dir.path().exists());
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
        vec![RedisKeyValueStore],
        vec!["custom"],
    )
    .await
}

#[tokio::test]
async fn misconfigured_spin_key_value_fails() -> anyhow::Result<()> {
    let runtime_config = toml::toml! {
        [key_value_store.custom]
        type = "spin"
        path = "/$$&/bad/path/foo.db"
    };
    assert!(run_test_with_config_and_stores_for_label(
        Some(runtime_config),
        vec![SpinKeyValueStore::new(
            std::env::current_dir().context("failed to get current directory")?
        )],
        vec!["custom"]
    )
    .await
    .is_err());
    Ok(())
}

#[tokio::test]
async fn multiple_custom_key_value_uses_first_store() -> anyhow::Result<()> {
    let tmp_dir = tempdir::TempDir::new("example")?;
    let mut test_resolver = RuntimeConfigResolver::new();
    test_resolver.register_store_type(RedisKeyValueStore)?;
    test_resolver.register_store_type(SpinKeyValueStore::new(tmp_dir.path().to_owned()))?;
    let test_resolver = Arc::new(test_resolver);
    let factors = TestFactors {
        key_value: KeyValueFactor::new(test_resolver.clone()),
    };
    let env = TestEnvironment::new(factors)
        .extend_manifest(toml! {
            [component.test-component]
            source = "does-not-exist.wasm"
            key_value_stores = ["custom"]
        })
        .runtime_config(TomlConfig::new(
            test_resolver,
            Some(toml::toml! {
                [key_value_store.custom]
                type = "spin"
                path = "custom.db"

                [key_value_store.custom]
                type = "redis"
                url = "redis://localhost:6379"
            }),
        ))?;
    let state = env.build_instance_state().await?;

    assert_eq!(
        state.key_value.allowed_stores(),
        &["custom".into()].into_iter().collect::<HashSet<_>>()
    );
    // Check that the first store in the config was chosen by verifying the existence of the DB directory
    assert!(tmp_dir.path().exists());
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
        self.resolver.resolve_from_toml(&self.toml)
    }
}

impl RuntimeConfigSourceFinalizer for TomlConfig {
    fn finalize(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}
