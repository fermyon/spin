pub mod delegating_resolver;
mod runtime_config;
mod store;
pub use delegating_resolver::{DelegatingRuntimeConfigResolver, StoreConfig};

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use anyhow::ensure;
use runtime_config::{RuntimeConfig, RuntimeConfigResolver};
use spin_factors::{
    ConfigureAppContext, Factor, FactorInstanceBuilder, InitContext, InstanceBuilders,
    PrepareContext, RuntimeFactors,
};
use spin_key_value::{
    CachingStoreManager, DefaultManagerGetter, DelegatingStoreManager, KeyValueDispatch,
    StoreManager, KEY_VALUE_STORES_KEY,
};
pub use store::MakeKeyValueStore;

/// A factor that provides key-value storage.
pub struct KeyValueFactor<R> {
    /// Resolves runtime configuration into store managers.
    runtime_config_resolver: Arc<R>,
}

impl<R> KeyValueFactor<R> {
    /// Create a new KeyValueFactor.  
    ///  
    /// The `runtime_config_resolver` is used to resolve runtime configuration into store managers.
    pub fn new(runtime_config_resolver: R) -> Self {
        Self {
            runtime_config_resolver: Arc::new(runtime_config_resolver),
        }
    }
}

impl<R: RuntimeConfigResolver + 'static> Factor for KeyValueFactor<R> {
    type RuntimeConfig = RuntimeConfig<R::Config>;
    type AppState = AppState;
    type InstanceBuilder = InstanceBuilder;

    fn init<T: Send + 'static>(&mut self, mut ctx: InitContext<T, Self>) -> anyhow::Result<()> {
        ctx.link_bindings(spin_world::v1::key_value::add_to_linker)?;
        ctx.link_bindings(spin_world::v2::key_value::add_to_linker)?;
        Ok(())
    }

    fn configure_app<T: RuntimeFactors>(
        &self,
        mut ctx: ConfigureAppContext<T, Self>,
    ) -> anyhow::Result<Self::AppState> {
        // Build store manager from runtime config
        let mut store_managers: HashMap<String, Arc<dyn StoreManager>> = HashMap::new();
        if let Some(runtime_config) = ctx.take_runtime_config() {
            for (store_label, config) in runtime_config.store_configs {
                if let std::collections::hash_map::Entry::Vacant(e) =
                    store_managers.entry(store_label)
                {
                    // Only add manager for labels that are not already configured. Runtime config
                    // takes top-down precedence.
                    let store = self.runtime_config_resolver.get_store(config)?;
                    e.insert(store);
                }
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
    /// The store manager for the app.
    ///
    /// This is a cache around a delegating store manager. For `get` requests,
    /// first checks the cache before delegating to the underlying store
    /// manager.
    store_manager: Arc<AppStoreManager>,
    /// The allowed stores for each component.
    ///
    /// This is a map from component ID to the set of store labels that the
    /// component is allowed to use.
    component_allowed_stores: HashMap<String, HashSet<String>>,
}

pub struct InstanceBuilder {
    /// The store manager for the app.
    ///
    /// This is a cache around a delegating store manager. For `get` requests,
    /// first checks the cache before delegating to the underlying store
    /// manager.
    store_manager: Arc<AppStoreManager>,
    /// The allowed stores for this component instance.
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
