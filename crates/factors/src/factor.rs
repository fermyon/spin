use std::any::Any;

use crate::{
    prepare::FactorInstanceBuilder, runtime_config::RuntimeConfigTracker, App, Error,
    FactorRuntimeConfig, InstanceBuilders, Linker, PrepareContext, RuntimeConfigSource,
    RuntimeFactors,
};

/// A contained (i.e., "factored") piece of runtime functionality.
pub trait Factor: Any + Sized {
    /// The particular runtime configuration relevant to this factor.
    ///
    /// Runtime configuration allows for user provided customization of the
    /// factor's behavior on a per app basis.
    type RuntimeConfig: FactorRuntimeConfig;

    /// The application state of this factor.
    ///
    /// This state *may* be cached by the runtime across multiple requests.
    type AppState;

    /// The builder of instance state for this factor.
    type InstanceBuilder: FactorInstanceBuilder;

    /// Initializes this `Factor` for a runtime once at runtime startup.
    ///
    /// This will be called at most once, before any call to [`FactorInstanceBuilder::new`].
    /// `InitContext` provides access to a wasmtime `Linker`, so this is where any bindgen
    /// `add_to_linker` calls go.
    fn init<T: RuntimeFactors>(&mut self, mut ctx: InitContext<T, Self>) -> anyhow::Result<()> {
        _ = &mut ctx;
        Ok(())
    }

    /// Performs factor-specific validation and configuration for the given
    /// [`App`].
    ///
    /// `ConfigureAppContext` gives access to:
    /// - The `spin_app::App`
    /// - This factors's `RuntimeConfig`
    /// - The `AppState` for any factors configured before this one
    ///
    /// A runtime may - but is not required to - reuse the returned config
    /// across multiple instances.
    ///
    /// This method may be called without any call to `init` or prepare in
    /// cases where only validation is needed (e.g., `spin doctor`).
    fn configure_app<T: RuntimeFactors>(
        &self,
        ctx: ConfigureAppContext<T, Self>,
    ) -> anyhow::Result<Self::AppState>;

    /// Creates a new `FactorInstanceBuilder`, which will later build per-instance
    /// state for this factor.
    ///
    /// This method is given access to the app component being instantiated and
    /// to any other factors' instance builders that have already been prepared.
    /// As such this is primary place for inter-factor dependencies.
    fn prepare<T: RuntimeFactors>(
        &self,
        ctx: PrepareContext<Self>,
        _builders: &mut InstanceBuilders<T>,
    ) -> anyhow::Result<Self::InstanceBuilder>;
}

/// The instance state of the given [`Factor`] `F`.
pub type FactorInstanceState<F> =
    <<F as Factor>::InstanceBuilder as FactorInstanceBuilder>::InstanceState;

pub(crate) type GetDataFn<Facts, F> =
    fn(&mut <Facts as RuntimeFactors>::InstanceState) -> &mut FactorInstanceState<F>;

/// An InitContext is passed to [`Factor::init`], giving access to the global
/// common [`wasmtime::component::Linker`].
pub struct InitContext<'a, T: RuntimeFactors, F: Factor> {
    pub(crate) linker: &'a mut Linker<T>,
    pub(crate) get_data: GetDataFn<T, F>,
}

impl<'a, T: RuntimeFactors, F: Factor> InitContext<'a, T, F> {
    #[doc(hidden)]
    pub fn new(linker: &'a mut Linker<T>, get_data: GetDataFn<T, F>) -> Self {
        Self { linker, get_data }
    }

    /// Returns a mutable reference to the [`wasmtime::component::Linker`].
    pub fn linker(&mut self) -> &mut Linker<T> {
        self.linker
    }

    /// Returns a function that can be used to get the instance state for this factor.
    pub fn get_data_fn(&self) -> GetDataFn<T, F> {
        self.get_data
    }

    /// Convenience method to link a binding to the linker.
    pub fn link_bindings(
        &mut self,
        add_to_linker: impl Fn(
            &mut Linker<T>,
            fn(&mut T::InstanceState) -> &mut FactorInstanceState<F>,
        ) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        add_to_linker(self.linker, self.get_data)
    }
}

pub struct ConfigureAppContext<'a, T: RuntimeFactors, F: Factor> {
    app: &'a App,
    app_state: &'a T::AppState,
    runtime_config: Option<F::RuntimeConfig>,
}

impl<'a, T: RuntimeFactors, F: Factor> ConfigureAppContext<'a, T, F> {
    #[doc(hidden)]
    pub fn new<S: RuntimeConfigSource>(
        app: &'a App,
        app_state: &'a T::AppState,
        runtime_config_tracker: &mut RuntimeConfigTracker<S>,
    ) -> crate::Result<Self> {
        let runtime_config = runtime_config_tracker.get_config::<F>()?;
        Ok(Self {
            app,
            app_state,
            runtime_config,
        })
    }

    /// Get the [`App`] being configured.
    pub fn app(&self) -> &App {
        self.app
    }

    /// Get the app state related to the given factor.
    pub fn app_state<U: Factor>(&self) -> crate::Result<&U::AppState> {
        T::app_state::<U>(self.app_state).ok_or(Error::no_such_factor::<U>())
    }

    /// Get a reference to the runtime configuration for the given factor.
    pub fn runtime_config(&self) -> Option<&F::RuntimeConfig> {
        self.runtime_config.as_ref()
    }

    /// Take ownership of the runtime configuration for the given factor.
    pub fn take_runtime_config(&mut self) -> Option<F::RuntimeConfig> {
        self.runtime_config.take()
    }
}

#[doc(hidden)]
pub struct ConfiguredApp<T: RuntimeFactors> {
    app: App,
    app_state: T::AppState,
}

impl<T: RuntimeFactors> ConfiguredApp<T> {
    #[doc(hidden)]
    pub fn new(app: App, app_state: T::AppState) -> Self {
        Self { app, app_state }
    }

    /// Get the configured [`App`].
    pub fn app(&self) -> &App {
        &self.app
    }

    /// Get the configured app's state related to the given factor.
    pub fn app_state<U: Factor>(&self) -> crate::Result<&U::AppState> {
        T::app_state::<U>(&self.app_state).ok_or(Error::no_such_factor::<U>())
    }
}
