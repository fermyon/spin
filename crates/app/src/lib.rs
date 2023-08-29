//! Spin internal application interfaces
//!
//! This crate contains interfaces to Spin application configuration to be used
//! by crates that implement Spin execution environments: trigger executors and
//! host components, in particular.

#![deny(missing_docs)]

mod host_component;
pub mod locked;
mod metadata;
pub mod values;

use ouroboros::self_referencing;
use serde::Deserialize;
use spin_core::{wasmtime, Engine, EngineBuilder, HostComponentDataHandle, StoreBuilder};

use host_component::DynamicHostComponents;
use locked::{ContentPath, LockedApp, LockedComponent, LockedComponentSource, LockedTrigger};
use metadata::MetadataExt;

pub use async_trait::async_trait;
pub use host_component::DynamicHostComponent;
pub use locked::Variable;
pub use metadata::MetadataKey;

/// MetadataKey for extracting the application name.
pub const APP_NAME_KEY: MetadataKey = MetadataKey::new("name");
/// MetadataKey for extracting the application version.
pub const APP_VERSION_KEY: MetadataKey = MetadataKey::new("version");
/// MetadataKey for extracting the bindle version.
pub const BINDLE_VERSION_KEY: MetadataKey = MetadataKey::new("bindle_version");
/// MetadataKey for extracting the OCI image digest.
pub const OCI_IMAGE_DIGEST_KEY: MetadataKey = MetadataKey::new("oci_image_digest");

/// A trait for implementing the low-level operations needed to load an [`App`].
// TODO(lann): Should this migrate to spin-loader?
#[async_trait]
pub trait Loader {
    /// Called with an implementation-defined `uri` pointing to some
    /// representation of a [`LockedApp`], which will be loaded.
    async fn load_app(&self, uri: &str) -> anyhow::Result<LockedApp>;

    /// Called with a [`LockedComponentSource`] pointing to a Wasm component
    /// binary, which will be loaded.
    async fn load_component(
        &self,
        engine: &wasmtime::Engine,
        source: &LockedComponentSource,
    ) -> anyhow::Result<spin_core::Component>;

    /// Called with a [`LockedComponentSource`] pointing to a Wasm module
    /// binary, which will be loaded.
    async fn load_module(
        &self,
        engine: &wasmtime::Engine,
        source: &LockedComponentSource,
    ) -> anyhow::Result<spin_core::Module>;

    /// Called with an [`AppComponent`]; any `files` configured with the
    /// component should be "mounted" into the `store_builder`, via e.g.
    /// [`StoreBuilder::read_only_preopened_dir`].
    async fn mount_files(
        &self,
        store_builder: &mut StoreBuilder,
        component: &AppComponent,
    ) -> anyhow::Result<()>;
}

/// An `AppLoader` holds an implementation of [`Loader`] along with
/// [`DynamicHostComponent`]s configuration.
pub struct AppLoader {
    inner: Box<dyn Loader + Send + Sync>,
    dynamic_host_components: DynamicHostComponents,
}

impl AppLoader {
    /// Creates a new [`AppLoader`].
    pub fn new(loader: impl Loader + Send + Sync + 'static) -> Self {
        Self {
            inner: Box::new(loader),
            dynamic_host_components: Default::default(),
        }
    }

    /// Adds a [`DynamicHostComponent`] to the given [`EngineBuilder`] and
    /// configures this [`AppLoader`] to update it on component instantiation.
    ///
    /// This calls [`EngineBuilder::add_host_component`] for you; it should not
    /// be called separately.
    pub fn add_dynamic_host_component<T: Send + Sync, DHC: DynamicHostComponent>(
        &mut self,
        engine_builder: &mut EngineBuilder<T>,
        host_component: DHC,
    ) -> anyhow::Result<HostComponentDataHandle<DHC>> {
        self.dynamic_host_components
            .add_dynamic_host_component(engine_builder, host_component)
    }

    /// Loads an [`App`] from the given `Loader`-implementation-specific `uri`.
    pub async fn load_app(&self, uri: String) -> Result<App> {
        let locked = self
            .inner
            .load_app(&uri)
            .await
            .map_err(Error::LoaderError)?;
        let app = App {
            loader: self,
            uri,
            locked,
        };
        self.dynamic_host_components
            .validate_app(&app)
            .map_err(Error::ValidationError)?;
        Ok(app)
    }

    /// Loads an [`OwnedApp`] from the given `Loader`-implementation-specific
    /// `uri`; the [`OwnedApp`] takes ownership of this [`AppLoader`].
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

impl OwnedApp {
    /// Returns a reference to the owned [`App`].
    pub fn borrowed(&self) -> &App {
        self.borrow_app()
    }
}

// Implementation detail of [`App::inert`]; "sealed" to prevent external impls.
mod private {
    pub trait MaybeLoader {}
}
use private::MaybeLoader;

impl MaybeLoader for AppLoader {}

/// An `App` holds loaded configuration for a Spin application.
///
/// Note: The `L: MaybeLoader` param is an implementation detail to support the
/// [`App::inert`] constructor.
#[derive(Debug)]
pub struct App<'a, L: MaybeLoader = AppLoader> {
    loader: &'a L,
    uri: String,
    locked: LockedApp,
}

impl<'a, L: MaybeLoader> App<'a, L> {
    /// Deserializes typed metadata for this app.
    ///
    /// Returns `Ok(None)` if there is no metadata for the given `key` and an
    /// `Err` only if there _is_ a value for the `key` but the typed
    /// deserialization failed.
    pub fn get_metadata<'this, T: Deserialize<'this>>(
        &'this self,
        key: MetadataKey<T>,
    ) -> Result<Option<T>> {
        self.locked.get_metadata(key)
    }

    /// Deserializes typed metadata for this app.
    ///
    /// Like [`App::get_metadata`], but returns an error if there is
    /// no metadata for the given `key`.
    pub fn require_metadata<'this, T: Deserialize<'this>>(
        &'this self,
        key: MetadataKey<T>,
    ) -> Result<T> {
        self.locked.require_metadata(key)
    }

    /// Returns an iterator of custom config [`Variable`]s defined for this app.
    pub fn variables(&self) -> impl Iterator<Item = (&String, &Variable)> {
        self.locked.variables.iter()
    }

    /// Returns an iterator of [`AppComponent`]s defined for this app.
    pub fn components(&self) -> impl Iterator<Item = AppComponent<'_, L>> {
        self.locked
            .components
            .iter()
            .map(|locked| AppComponent { app: self, locked })
    }

    /// Returns the [`AppComponent`] with the given `component_id`, or `None`
    /// if it doesn't exist.
    pub fn get_component(&self, component_id: &str) -> Option<AppComponent<'_, L>> {
        self.components()
            .find(|component| component.locked.id == component_id)
    }

    /// Returns an iterator of [`AppTrigger`]s defined for this app.
    pub fn triggers(&self) -> impl Iterator<Item = AppTrigger<'_, L>> {
        self.locked
            .triggers
            .iter()
            .map(|locked| AppTrigger { app: self, locked })
    }

    /// Returns an iterator of [`AppTrigger`]s defined for this app with
    /// the given `trigger_type`.
    pub fn triggers_with_type(
        &'a self,
        trigger_type: &'a str,
    ) -> impl Iterator<Item = AppTrigger<'_, L>> {
        self.triggers()
            .filter(move |trigger| trigger.locked.trigger_type == trigger_type)
    }
}

impl<'a> App<'a> {
    /// Returns a [`Loader`]-implementation-specific URI for this app.
    pub fn uri(&self) -> &str {
        &self.uri
    }
}

/// Used in the return type of [`App::inert`] to prevent the use of methods
/// that require an [`AppLoader`].
pub struct InertLoader;
impl MaybeLoader for InertLoader {}

impl App<'static, InertLoader> {
    /// Return an "inert" App which does not have an associated [`AppLoader`]
    /// and cannot be used to instantiate components.
    pub fn inert(locked: LockedApp) -> App<'static, InertLoader> {
        App {
            loader: &InertLoader,
            uri: "".into(),
            locked,
        }
    }
}

/// An `AppComponent` holds configuration for a Spin application component.
pub struct AppComponent<'a, L: MaybeLoader = AppLoader> {
    /// The app this component belongs to.
    pub app: &'a App<'a, L>,
    locked: &'a LockedComponent,
}

impl<'a, L: MaybeLoader> AppComponent<'a, L> {
    /// Returns this component's app-unique ID.
    pub fn id(&self) -> &str {
        &self.locked.id
    }

    /// Returns this component's Wasm component or module source.
    pub fn source(&self) -> &LockedComponentSource {
        &self.locked.source
    }

    /// Returns an iterator of [`ContentPath`]s for this component's configured
    /// "directory mounts".
    pub fn files(&self) -> std::slice::Iter<ContentPath> {
        self.locked.files.iter()
    }

    /// Deserializes typed metadata for this component.
    ///
    /// Returns `Ok(None)` if there is no metadata for the given `key` and an
    /// `Err` only if there _is_ a value for the `key` but the typed
    /// deserialization failed.
    pub fn get_metadata<T: Deserialize<'a>>(&self, key: MetadataKey<T>) -> Result<Option<T>> {
        self.locked.metadata.get_typed(key)
    }

    /// Deserializes typed metadata for this component.
    ///
    /// Like [`AppComponent::get_metadata`], but returns an error if there is
    /// no metadata for the given `key`.
    pub fn require_metadata<'this, T: Deserialize<'this>>(
        &'this self,
        key: MetadataKey<T>,
    ) -> Result<T> {
        self.locked.metadata.require_typed(key)
    }

    /// Returns an iterator of custom config values for this component.
    pub fn config(&self) -> impl Iterator<Item = (&String, &String)> {
        self.locked.config.iter()
    }
}

impl<'a> AppComponent<'a> {
    /// Loads and returns the [`spin_core::Component`] for this component.
    pub async fn load_component<T: Send + Sync>(
        &self,
        engine: &Engine<T>,
    ) -> Result<spin_core::Component> {
        self.app
            .loader
            .inner
            .load_component(engine.as_ref(), &self.locked.source)
            .await
            .map_err(Error::LoaderError)
    }

    /// Loads and returns the [`spin_core::Module`] for this component.
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

    /// Updates the given [`StoreBuilder`] with configuration for this component.
    ///
    /// In particular, the WASI 'env' and "preloaded dirs" are set up, and any
    /// [`DynamicHostComponent`]s associated with the source [`AppLoader`] are
    /// configured.
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

/// An `AppTrigger` holds configuration for a Spin application trigger.
pub struct AppTrigger<'a, L: MaybeLoader = AppLoader> {
    /// The app this trigger belongs to.
    pub app: &'a App<'a, L>,
    locked: &'a LockedTrigger,
}

impl<'a, L: MaybeLoader> AppTrigger<'a, L> {
    /// Returns this trigger's app-unique ID.
    pub fn id(&self) -> &str {
        &self.locked.id
    }

    /// Returns the Trigger's type.
    pub fn trigger_type(&self) -> &str {
        &self.locked.trigger_type
    }

    /// Deserializes this trigger's configuration into a typed value.
    pub fn typed_config<Config: Deserialize<'a>>(&self) -> Result<Config> {
        Ok(Config::deserialize(&self.locked.trigger_config)?)
    }

    /// Returns a reference to the [`AppComponent`] configured for this trigger.
    ///
    /// This is a convenience wrapper that looks up the component based on the
    /// 'component' metadata value which is conventionally a component ID.
    pub fn component(&self) -> Result<AppComponent<'a, L>> {
        let id = &self.locked.id;
        let common_config: CommonTriggerConfig = self.typed_config()?;
        let component_id = common_config.component.ok_or_else(|| {
            Error::MetadataError(format!("trigger {id:?} missing 'component' config field"))
        })?;
        self.app.get_component(&component_id).ok_or_else(|| {
            Error::MetadataError(format!(
                "missing component {component_id:?} configured for trigger {id:?}"
            ))
        })
    }
}

#[derive(Deserialize)]
struct CommonTriggerConfig {
    component: Option<String>,
}

/// Type alias for a [`Result`]s with [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

/// Errors returned by methods in this crate.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An error propagated from the [`spin_core`] crate.
    #[error("spin core error: {0:#}")]
    CoreError(#[source] anyhow::Error),
    /// An error from a [`DynamicHostComponent`].
    #[error("host component error: {0:#}")]
    HostComponentError(#[source] anyhow::Error),
    /// An error from a [`Loader`] implementation.
    #[error(transparent)]
    LoaderError(anyhow::Error),
    /// An error indicating missing or unexpected metadata.
    #[error("metadata error: {0}")]
    MetadataError(String),
    /// An error indicating failed JSON (de)serialization.
    #[error("json error: {0}")]
    JsonError(#[from] serde_json::Error),
    /// A validation error that can be presented directly to the user.
    #[error(transparent)]
    ValidationError(anyhow::Error),
}
