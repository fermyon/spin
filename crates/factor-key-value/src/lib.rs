mod runtime_config;
pub mod delegating_resolver;
mod store;

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use anyhow::ensure;
use runtime_config::RuntimeConfig;
use spin_factors::{
    ConfigureAppContext, Factor, FactorInstanceBuilder, InitContext, InstanceBuilders,
    PrepareContext, RuntimeFactors,
};
use spin_key_value::{
    CachingStoreManager, DefaultManagerGetter, DelegatingStoreManager, KeyValueDispatch,
    StoreManager, KEY_VALUE_STORES_KEY,
};
pub use store::MakeKeyValueStore;

pub struct KeyValueFactor {
    runtime_config_resolver: Arc<dyn runtime_config::RuntimeConfigResolver>,
}

impl KeyValueFactor {
    pub fn new(
        runtime_config_resolver: impl runtime_config::RuntimeConfigResolver + 'static,
    ) -> Self {
        Self {
            runtime_config_resolver: Arc::new(runtime_config_resolver),
        }
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
        let mut store_managers: HashMap<String, Arc<dyn StoreManager>> = HashMap::new();
        if let Some(runtime_config) = ctx.take_runtime_config() {
            for (
                store_label,
                runtime_config::StoreConfig {
                    type_: store_kind,
                    config,
                },
            ) in runtime_config.store_configs
            {
                let store = self
                    .runtime_config_resolver
                    .get_store(&store_kind, config)?;
                store_managers.insert(store_label, store);
            }
        }
        let resolver_clone = self.runtime_config_resolver.clone();
        let default_fn: DefaultManagerGetter =
            Arc::new(move |label| resolver_clone.default_store(label));

        let delegating_manager = DelegatingStoreManager::new(store_managers, default_fn);
        let caching_manager = CachingStoreManager::new(delegating_manager);
        let store_manager_manager = Arc::new(caching_manager);

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
                    store_manager_manager.is_defined(label)
                        || self.runtime_config_resolver.default_store(label).is_some(),
                    "unknown key_value_stores label {label:?} for component {component_id:?}"
                );
            }
            component_allowed_stores.insert(component_id, key_value_stores);
            // TODO: warn (?) on unused store?
        }

        Ok(AppState {
            store_manager: store_manager_manager,
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
