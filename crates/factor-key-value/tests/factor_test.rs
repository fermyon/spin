use anyhow::bail;
use spin_core::async_trait;
use spin_factor_key_value::{KeyValueFactor, RuntimeConfig, Store, StoreManager};
use spin_factors::RuntimeFactors;
use spin_factors_test::{toml, TestEnvironment};
use spin_world::v2::key_value::{Error, HostStore};
use std::{collections::HashSet, sync::Arc};

#[derive(RuntimeFactors)]
struct TestFactors {
    key_value: KeyValueFactor,
}

impl Into<TestFactorsRuntimeConfig> for RuntimeConfig {
    fn into(self) -> TestFactorsRuntimeConfig {
        TestFactorsRuntimeConfig {
            key_value: Some(self),
        }
    }
}

#[tokio::test]
async fn works_when_allowed_store_is_defined() -> anyhow::Result<()> {
    let mut runtime_config = RuntimeConfig::default();
    runtime_config.add_store_manager("default".into(), mock_store_manager());
    let factors = TestFactors {
        key_value: KeyValueFactor::new(),
    };
    let env = TestEnvironment::new(factors).extend_manifest(toml! {
        [component.test-component]
        source = "does-not-exist.wasm"
        key_value_stores = ["default"]
    });
    let mut state = env
        .runtime_config(runtime_config)?
        .build_instance_state()
        .await?;

    assert_eq!(
        state.key_value.allowed_stores(),
        &["default".into()].into_iter().collect::<HashSet<_>>()
    );

    assert!(state.key_value.open("default".to_owned()).await?.is_ok());
    Ok(())
}

#[tokio::test]
async fn errors_when_store_is_not_defined() -> anyhow::Result<()> {
    let runtime_config = RuntimeConfig::default();
    let factors = TestFactors {
        key_value: KeyValueFactor::new(),
    };
    let env = TestEnvironment::new(factors).extend_manifest(toml! {
        [component.test-component]
        source = "does-not-exist.wasm"
        key_value_stores = ["default"]
    });
    let Err(err) = env
        .runtime_config(runtime_config)?
        .build_instance_state()
        .await
    else {
        bail!("expected instance build to fail but it didn't");
    };

    assert!(err
        .to_string()
        .contains(r#"unknown key_value_stores label "default""#));

    Ok(())
}

#[tokio::test]
async fn errors_when_store_is_not_allowed() -> anyhow::Result<()> {
    let mut runtime_config = RuntimeConfig::default();
    runtime_config.add_store_manager("default".into(), mock_store_manager());
    let factors = TestFactors {
        key_value: KeyValueFactor::new(),
    };
    let env = TestEnvironment::new(factors).extend_manifest(toml! {
        [component.test-component]
        source = "does-not-exist.wasm"
        key_value_stores = []
    });
    let mut state = env
        .runtime_config(runtime_config)?
        .build_instance_state()
        .await?;

    assert_eq!(state.key_value.allowed_stores(), &HashSet::new());

    assert!(matches!(
        state.key_value.open("default".to_owned()).await?,
        Err(Error::AccessDenied)
    ));

    Ok(())
}

fn mock_store_manager() -> Arc<dyn StoreManager> {
    Arc::new(MockStoreManager)
}

struct MockStoreManager;

#[async_trait]
impl StoreManager for MockStoreManager {
    async fn get(&self, name: &str) -> Result<Arc<dyn Store>, Error> {
        let _ = name;
        Ok(Arc::new(MockStore))
    }

    fn is_defined(&self, store_name: &str) -> bool {
        let _ = store_name;
        todo!()
    }
}

struct MockStore;

#[async_trait]
impl Store for MockStore {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, Error> {
        let _ = key;
        todo!()
    }
    async fn set(&self, key: &str, value: &[u8]) -> Result<(), Error> {
        let _ = (key, value);
        todo!()
    }
    async fn delete(&self, key: &str) -> Result<(), Error> {
        let _ = key;
        todo!()
    }
    async fn exists(&self, key: &str) -> Result<bool, Error> {
        let _ = key;
        todo!()
    }
    async fn get_keys(&self) -> Result<Vec<String>, Error> {
        todo!()
    }
}

// async fn run_test_with_config_and_stores_for_label(
//     runtime_config: Option<toml::Table>,
//     store_types: Vec<impl MakeKeyValueStore>,
//     labels: Vec<&str>,
// ) -> anyhow::Result<TestFactorsInstanceState> {
//     let mut test_resolver = RuntimeConfigResolver::new();
//     for store_type in store_types {
//         test_resolver.register_store_type(store_type)?;
//     }
//     let test_resolver = Arc::new(test_resolver);
//     let factors = TestFactors {
//         key_value: KeyValueFactor::new(),
//     };
//     let labels_clone = labels.clone();
//     let env = TestEnvironment::new(factors)
//         .extend_manifest(toml! {
//             [component.test-component]
//             source = "does-not-exist.wasm"
//             key_value_stores = labels_clone
//         })
//         .runtime_config(TomlConfig::new(test_resolver, runtime_config))?;
//     let state = env.build_instance_state().await?;
//     assert_eq!(
//         labels,
//         state.key_value.allowed_stores().iter().collect::<Vec<_>>()
//     );

//     Ok(state)
// }

// #[tokio::test]
// async fn overridden_default_key_value_works() -> anyhow::Result<()> {
//     let runtime_config = toml::toml! {
//         [key_value_store.default]
//         type = "redis"
//         url = "redis://localhost:6379"
//     };
//     run_test_with_config_and_stores_for_label(
//         Some(runtime_config),
//         vec![RedisKeyValueStore::new()],
//         vec!["default"],
//     )
//     .await?;
//     Ok(())
// }

// #[tokio::test]
// async fn custom_spin_key_value_works() -> anyhow::Result<()> {
//     let runtime_config = toml::toml! {
//         [key_value_store.custom]
//         type = "spin"
//     };
//     run_test_with_config_and_stores_for_label(
//         Some(runtime_config),
//         vec![SpinKeyValueStore::new(None)],
//         vec!["custom"],
//     )
//     .await?;
//     Ok(())
// }

// #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
// async fn custom_spin_key_value_works_with_absolute_path() -> anyhow::Result<()> {
//     let tmp_dir = tempfile::TempDir::with_prefix("example")?;
//     let db_path = tmp_dir.path().join("foo/custom.db");
//     // Check that the db does not exist yet - it will exist by the end of the test
//     assert!(!db_path.exists());

//     let path_str = db_path.to_str().unwrap();
//     let runtime_config = toml::toml! {
//         [key_value_store.custom]
//         type = "spin"
//         path = path_str
//     };
//     let mut state = run_test_with_config_and_stores_for_label(
//         Some(runtime_config),
//         vec![SpinKeyValueStore::new(Some(
//             std::env::current_dir().context("failed to get current directory")?,
//         ))],
//         vec!["custom"],
//     )
//     .await?;

//     // Actually et a key since store creation is lazy
//     let store = state.key_value.open("custom".to_owned()).await??;
//     let _ = state.key_value.get(store, "foo".to_owned()).await??;

//     // Check that the parent has been created
//     assert!(db_path.exists());
//     Ok(())
// }

// #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
// async fn custom_spin_key_value_works_with_relative_path() -> anyhow::Result<()> {
//     let tmp_dir = tempfile::TempDir::with_prefix("example")?;
//     let db_path = tmp_dir.path().join("custom.db");
//     // Check that the db does not exist yet - it will exist by the end of the test
//     assert!(!db_path.exists());

//     let runtime_config = toml::toml! {
//         [key_value_store.custom]
//         type = "spin"
//         path = "custom.db"
//     };
//     let mut state = run_test_with_config_and_stores_for_label(
//         Some(runtime_config),
//         vec![SpinKeyValueStore::new(Some(tmp_dir.path().to_owned()))],
//         vec!["custom"],
//     )
//     .await?;

//     // Actually et a key since store creation is lazy
//     let store = state.key_value.open("custom".to_owned()).await??;
//     let _ = state.key_value.get(store, "foo".to_owned()).await??;

//     // Check that the correct store in the config was chosen by verifying the existence of the DB
//     assert!(db_path.exists());
//     Ok(())
// }

// #[tokio::test]
// async fn custom_redis_key_value_works() -> anyhow::Result<()> {
//     let runtime_config = toml::toml! {
//         [key_value_store.custom]
//         type = "redis"
//         url = "redis://localhost:6379"
//     };
//     run_test_with_config_and_stores_for_label(
//         Some(runtime_config),
//         vec![RedisKeyValueStore::new()],
//         vec!["custom"],
//     )
//     .await?;
//     Ok(())
// }

// #[tokio::test]
// async fn misconfigured_spin_key_value_fails() -> anyhow::Result<()> {
//     let tmp_dir = tempfile::TempDir::with_prefix("example")?;
//     let runtime_config = toml::toml! {
//         [key_value_store.custom]
//         type = "spin"
//         path = "/$$&/bad/path/foo.db"
//     };
//     let result = run_test_with_config_and_stores_for_label(
//         Some(runtime_config),
//         vec![SpinKeyValueStore::new(Some(tmp_dir.path().to_owned()))],
//         vec!["custom"],
//     )
//     .await;
//     // TODO(rylev): This only fails on my machine due to a read-only file system error.
//     // We should consider adding a check for the error message.
//     assert!(result.is_err());
//     Ok(())
// }
