mod store;

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use anyhow::{bail, ensure};
use serde::Deserialize;
use spin_factors::{
    anyhow::{self, Context},
    ConfigureAppContext, Factor, FactorInstanceBuilder, FactorRuntimeConfig, InitContext,
    InstanceBuilders, PrepareContext, RuntimeFactors,
};
use spin_key_value::{
    CachingStoreManager, DelegatingStoreManager, KeyValueDispatch, StoreManager,
    KEY_VALUE_STORES_KEY,
};
use store::{store_from_toml_fn, StoreFromToml};

pub use store::MakeKeyValueStore;

#[derive(Default)]
pub struct KeyValueFactor {
    store_types: HashMap<&'static str, StoreFromToml>,
}

impl KeyValueFactor {
    pub fn add_store_type<T: MakeKeyValueStore>(&mut self, store_type: T) -> anyhow::Result<()> {
        if self
            .store_types
            .insert(T::RUNTIME_CONFIG_TYPE, store_from_toml_fn(store_type))
            .is_some()
        {
            bail!(
                "duplicate key value store type {:?}",
                T::RUNTIME_CONFIG_TYPE
            );
        }
        Ok(())
    }
}

impl Factor for KeyValueFactor {
    type RuntimeConfig = RuntimeConfig;
    type AppState = AppState;
    type InstanceBuilder = InstanceBuilder;

    fn init<Factors: RuntimeFactors>(
        &mut self,
        mut ctx: InitContext<Factors, Self>,
    ) -> anyhow::Result<()> {
        ctx.link_bindings(spin_world::v1::key_value::add_to_linker)?;
        ctx.link_bindings(spin_world::v2::key_value::add_to_linker)?;
        Ok(())
    }

    fn configure_app<T: RuntimeFactors>(
        &self,
        mut ctx: ConfigureAppContext<T, Self>,
    ) -> anyhow::Result<Self::AppState> {
        // Build StoreManager from runtime config
        let mut stores = HashMap::new();
        if let Some(runtime_config) = ctx.take_runtime_config() {
            for (label, StoreConfig { type_, config }) in runtime_config.store_configs {
                let store_maker = self
                    .store_types
                    .get(type_.as_str())
                    .with_context(|| format!("unknown key value store type {type_:?}"))?;
                let store = store_maker(config)?;
                stores.insert(label, store);
            }
        }
        let delegating_manager = DelegatingStoreManager::new(stores);
        let caching_manager = CachingStoreManager::new(delegating_manager);
        let store_manager = Arc::new(caching_manager);

        // Build component -> allowed stores map
        let mut component_allowed_stores = HashMap::new();
        for component in ctx.app().components() {
            let component_id = component.id().to_string();
            let key_value_stores = component
                .get_metadata(KEY_VALUE_STORES_KEY)?
                .unwrap_or_default()
                .into_iter()
                .collect::<HashSet<_>>();
            for label in &key_value_stores {
                // TODO: port nicer errors from KeyValueComponent (via error type?)
                ensure!(
                    store_manager.is_defined(label),
                    "unknown key_value_stores label {label:?} for component {component_id:?}"
                );
            }
            component_allowed_stores.insert(component_id, key_value_stores);
            // TODO: warn (?) on unused store?
        }

        Ok(AppState {
            store_manager,
            component_allowed_stores,
        })
    }

    fn prepare<T: RuntimeFactors>(
        &self,
        ctx: PrepareContext<Self>,
        _builders: &mut InstanceBuilders<T>,
    ) -> anyhow::Result<InstanceBuilder> {
        let app_state = ctx.app_state();
        let allowed_stores = app_state
            .component_allowed_stores
            .get(ctx.app_component().id())
            .expect("component should be in component_stores")
            .clone();
        Ok(InstanceBuilder {
            store_manager: app_state.store_manager.clone(),
            allowed_stores,
        })
    }
}

#[derive(Deserialize)]
#[serde(transparent)]
pub struct RuntimeConfig {
    store_configs: HashMap<String, StoreConfig>,
}

impl FactorRuntimeConfig for RuntimeConfig {
    const KEY: &'static str = "key_value_store";
}

#[derive(Deserialize)]
struct StoreConfig {
    #[serde(rename = "type")]
    type_: String,
    #[serde(flatten)]
    config: toml::Table,
}

type AppStoreManager = CachingStoreManager<DelegatingStoreManager>;

pub struct AppState {
    store_manager: Arc<AppStoreManager>,
    component_allowed_stores: HashMap<String, HashSet<String>>,
}

pub struct InstanceBuilder {
    store_manager: Arc<AppStoreManager>,
    allowed_stores: HashSet<String>,
}

impl FactorInstanceBuilder for InstanceBuilder {
    type InstanceState = KeyValueDispatch;

    fn build(self) -> anyhow::Result<Self::InstanceState> {
        let Self {
            store_manager,
            allowed_stores,
        } = self;
        let mut dispatch = KeyValueDispatch::new_with_capacity(u32::MAX);
        dispatch.init(allowed_stores, store_manager);
        Ok(dispatch)
    }
}
