mod host_component;
pub mod locked;
pub mod values;

use ouroboros::self_referencing;
use serde::Deserialize;
use spin_core::{wasmtime, Engine, EngineBuilder, StoreBuilder};

use host_component::DynamicHostComponents;
use locked::{ContentPath, LockedApp, LockedComponent, LockedComponentSource, LockedTrigger};

pub use async_trait::async_trait;
pub use host_component::DynamicHostComponent;
pub use locked::Variable;

// TODO(lann): Should this migrate to spin-loader?
#[async_trait]
pub trait Loader {
    async fn load_app(&self, uri: &str) -> anyhow::Result<LockedApp>;

    async fn load_module(
        &self,
        engine: &wasmtime::Engine,
        source: &LockedComponentSource,
    ) -> anyhow::Result<spin_core::Module>;

    async fn mount_files(
        &self,
        store_builder: &mut StoreBuilder,
        component: &AppComponent,
    ) -> anyhow::Result<()>;
}

pub struct AppLoader {
    inner: Box<dyn Loader + Send + Sync>,
    dynamic_host_components: DynamicHostComponents,
}

impl AppLoader {
    pub fn new(loader: impl Loader + Send + Sync + 'static) -> Self {
        Self {
            inner: Box::new(loader),
            dynamic_host_components: Default::default(),
        }
    }

    pub fn add_dynamic_host_component<T: Send + Sync, DHC: DynamicHostComponent>(
        &mut self,
        engine_builder: &mut EngineBuilder<T>,
        host_component: DHC,
    ) -> anyhow::Result<()> {
        self.dynamic_host_components
            .add_dynamic_host_component(engine_builder, host_component)
    }

    pub async fn load_app(&self, uri: String) -> Result<App> {
        let locked = self
            .inner
            .load_app(&uri)
            .await
            .map_err(Error::LoaderError)?;
        Ok(App {
            loader: self,
            uri,
            locked,
        })
    }

    pub async fn load_owned_app(self, uri: String) -> Result<OwnedApp> {
        OwnedApp::try_new_async(self, |loader| Box::pin(loader.load_app(uri))).await
    }
}

impl std::fmt::Debug for AppLoader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppLoader").finish()
    }
}

#[self_referencing]
#[derive(Debug)]
pub struct OwnedApp {
    loader: AppLoader,

    #[borrows(loader)]
    #[covariant]
    app: App<'this>,
}

impl std::ops::Deref for OwnedApp {
    type Target = App<'static>;

    fn deref(&self) -> &Self::Target {
        unsafe {
            // We know that App's lifetime param is for AppLoader, which is owned by self here.
            std::mem::transmute::<&App, &App<'static>>(self.borrow_app())
        }
    }
}

#[derive(Debug)]
pub struct App<'a> {
    loader: &'a AppLoader,
    uri: String,
    locked: LockedApp,
}

impl<'a> App<'a> {
    pub fn uri(&self) -> &str {
        &self.uri
    }

    pub fn get_metadata<'this, T: Deserialize<'this>>(&'this self, key: &str) -> Result<Option<T>> {
        self.locked
            .metadata
            .get(key)
            .map(|value| Ok(T::deserialize(value)?))
            .transpose()
    }

    pub fn require_metadata<'this, T: Deserialize<'this>>(&'this self, key: &str) -> Result<T> {
        self.get_metadata(key)?
            .ok_or_else(|| Error::ManifestError(format!("missing required {key:?}")))
    }

    pub fn variables(&self) -> impl Iterator<Item = (&String, &Variable)> {
        self.locked.variables.iter()
    }

    pub fn components(&self) -> impl Iterator<Item = AppComponent> {
        self.locked
            .components
            .iter()
            .map(|locked| AppComponent { app: self, locked })
    }

    pub fn get_component(&self, component_id: &str) -> Option<AppComponent> {
        self.components()
            .find(|component| component.locked.id == component_id)
    }

    pub fn triggers(&self) -> impl Iterator<Item = AppTrigger> {
        self.locked
            .triggers
            .iter()
            .map(|locked| AppTrigger { app: self, locked })
    }

    pub fn triggers_with_type(&'a self, trigger_type: &'a str) -> impl Iterator<Item = AppTrigger> {
        self.triggers()
            .filter(move |trigger| trigger.locked.trigger_type == trigger_type)
    }
}

pub struct AppComponent<'a> {
    pub app: &'a App<'a>,
    locked: &'a LockedComponent,
}

impl<'a> AppComponent<'a> {
    pub fn id(&self) -> &str {
        &self.locked.id
    }

    pub fn source(&self) -> &LockedComponentSource {
        &self.locked.source
    }

    pub fn files(&self) -> std::slice::Iter<ContentPath> {
        self.locked.files.iter()
    }

    pub fn get_metadata<T: Deserialize<'a>>(&self, key: &str) -> Result<Option<T>> {
        self.locked
            .metadata
            .get(key)
            .map(|value| {
                T::deserialize(value).map_err(|err| {
                    Error::ManifestError(format!(
                        "failed to deserialize {key:?} = {value:?}: {err:?}"
                    ))
                })
            })
            .transpose()
    }

    pub fn config(&self) -> impl Iterator<Item = (&String, &String)> {
        self.locked.config.iter()
    }

    pub async fn load_module<T: Send + Sync>(
        &self,
        engine: &Engine<T>,
    ) -> Result<spin_core::Module> {
        self.app
            .loader
            .inner
            .load_module(engine.as_ref(), &self.locked.source)
            .await
            .map_err(Error::LoaderError)
    }

    pub async fn apply_store_config(&self, builder: &mut StoreBuilder) -> Result<()> {
        builder.env(&self.locked.env).map_err(Error::CoreError)?;

        let loader = self.app.loader;
        loader
            .inner
            .mount_files(builder, self)
            .await
            .map_err(Error::LoaderError)?;

        loader
            .dynamic_host_components
            .update_data(builder.host_components_data(), self)
            .map_err(Error::HostComponentError)?;

        Ok(())
    }
}

pub struct AppTrigger<'a> {
    pub app: &'a App<'a>,
    locked: &'a LockedTrigger,
}

impl<'a> AppTrigger<'a> {
    pub fn id(&self) -> &str {
        &self.locked.id
    }

    pub fn trigger_type(&self) -> &str {
        &self.locked.trigger_type
    }

    pub fn component(&self) -> Result<AppComponent<'a>> {
        let component_id = self.locked.trigger_config.get("component").ok_or_else(|| {
            Error::ManifestError(format!(
                "trigger {:?} missing 'component' config field",
                self.locked.id
            ))
        })?;
        let component_id = component_id.as_str().ok_or_else(|| {
            Error::ManifestError(format!(
                "trigger {:?} 'component' field has unexpected value {:?}",
                self.locked.id, component_id
            ))
        })?;
        self.app.get_component(component_id).ok_or_else(|| {
            Error::ManifestError(format!(
                "missing component {:?} configured for trigger {:?}",
                component_id, self.locked.id
            ))
        })
    }

    pub fn typed_config<Config: Deserialize<'a>>(&self) -> Result<Config> {
        Ok(Config::deserialize(&self.locked.trigger_config)?)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("spin core error: {0:#}")]
    CoreError(anyhow::Error),
    #[error("host component error: {0:#}")]
    HostComponentError(anyhow::Error),
    #[error("loader error: {0:#}")]
    LoaderError(anyhow::Error),
    #[error("manifest error: {0}")]
    ManifestError(String),
    #[error("json error: {0}")]
    JsonError(#[from] serde_json::Error),
}
