mod host;
pub mod runtime_config;
mod util;

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use anyhow::ensure;
use host::KEY_VALUE_STORES_KEY;
use spin_factors::{
    ConfigureAppContext, Factor, FactorInstanceBuilder, InitContext, InstanceBuilders,
    PrepareContext, RuntimeFactors,
};
use util::{CachingStoreManager, DefaultManagerGetter};

pub use host::{log_error, Error, KeyValueDispatch, Store, StoreManager};
pub use runtime_config::RuntimeConfig;
pub use util::DelegatingStoreManager;

/// A factor that provides key-value storage.
pub struct KeyValueFactor {
    default_label_resolver: Arc<dyn DefaultLabelResolver>,
}

impl KeyValueFactor {
    /// Create a new KeyValueFactor.
    ///
    /// The `default_label_resolver` is used to resolve store managers for labels that
    /// are not defined in the runtime configuration.
    pub fn new(default_label_resolver: impl DefaultLabelResolver + 'static) -> Self {
        Self {
            default_label_resolver: Arc::new(default_label_resolver),
        }
    }
}

impl Factor for KeyValueFactor {
    type RuntimeConfig = RuntimeConfig;
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
        let store_managers = ctx.take_runtime_config().unwrap_or_default();
        let default_label_resolver = self.default_label_resolver.clone();
        let default_fn: DefaultManagerGetter =
            Arc::new(move |label| default_label_resolver.default(label));

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
                        || self.default_label_resolver.default(label).is_some(),
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

impl AppState {
    /// Returns the [`StoreManager::summary`] for the given store label.
    pub fn store_summary(&self, label: &str) -> Option<String> {
        self.store_manager.summary(label)
    }

    /// Returns true if the given store label is used by any component.
    pub fn store_is_used(&self, label: &str) -> bool {
        self.component_allowed_stores
            .values()
            .any(|stores| stores.contains(label))
    }

    /// Get a store by label.
    pub async fn get_store(&self, label: &str) -> Option<Arc<dyn Store>> {
        self.store_manager.get(label).await.ok()
    }
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

/// Resolves a label to a default [`StoreManager`].
pub trait DefaultLabelResolver: Send + Sync {
    /// If there is no runtime configuration for a given store label, return a default store manager.
    ///
    /// If `Option::None` is returned, the store is not allowed.
    fn default(&self, label: &str) -> Option<Arc<dyn StoreManager>>;
}

impl<T: DefaultLabelResolver> DefaultLabelResolver for Arc<T> {
    fn default(&self, label: &str) -> Option<Arc<dyn StoreManager>> {
        self.as_ref().default(label)
    }
}
