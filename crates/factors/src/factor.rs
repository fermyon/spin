use std::any::Any;

use wasmtime::component::{Linker, ResourceTable};

use crate::{prepare::FactorInstanceBuilder, App, Error, PrepareContext, RuntimeFactors};

/// A contained (i.e., "factored") piece of runtime functionality.
pub trait Factor: Any + Sized {
    /// The particular runtime configuration relevant to this factor.
    ///
    /// Runtime configuration allows for user-provided customization of the
    /// factor's behavior on a per-app basis.
    type RuntimeConfig;

    /// The application state of this factor.
    ///
    /// This state *may* be cached by the runtime across multiple requests.
    type AppState: Sync;

    /// The builder of instance state for this factor.
    type InstanceBuilder: FactorInstanceBuilder;

    /// Initializes this `Factor` for a runtime once at runtime startup.
    ///
    /// This will be called at most once, before any call to
    /// [`Factor::prepare`]. `InitContext` provides access to a wasmtime
    /// `Linker`, so this is where any bindgen `add_to_linker` calls go.
    ///
    /// The type parameter `T` here is the same as the [`wasmtime::Store`] type
    /// parameter `T`, which will contain the [`RuntimeFactors::InstanceState`].
    fn init<T: Send + 'static>(&mut self, mut ctx: InitContext<T, Self>) -> anyhow::Result<()> {
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
    /// across multiple instances. Because this method may be called
    /// per-instantiation, it should avoid any blocking operations that could
    /// unnecessarily delay execution.
    ///
    /// This method may be called without any call to `init` or `prepare` in
    /// cases where only validation is needed (e.g., `spin doctor`).
    fn configure_app<T: RuntimeFactors>(
        &self,
        ctx: ConfigureAppContext<T, Self>,
    ) -> anyhow::Result<Self::AppState>;

    /// Creates a new `FactorInstanceBuilder`, which will later build
    /// per-instance state for this factor.
    ///
    /// This method is given access to the app component being instantiated and
    /// to any other factors' instance builders that have already been prepared.
    /// As such, this is the primary place for inter-factor dependencies to be
    /// used.
    fn prepare<T: RuntimeFactors>(
        &self,
        ctx: PrepareContext<T, Self>,
    ) -> anyhow::Result<Self::InstanceBuilder>;
}

/// The instance state of the given [`Factor`] `F`.
pub type FactorInstanceState<F> =
    <<F as Factor>::InstanceBuilder as FactorInstanceBuilder>::InstanceState;

pub(crate) type GetDataFn<T, U> = fn(&mut T) -> &mut FactorInstanceState<U>;

pub(crate) type GetDataWithTableFn<T, U> =
    fn(&mut T) -> (&mut FactorInstanceState<U>, &mut ResourceTable);

/// An InitContext is passed to [`Factor::init`], giving access to the global
/// common [`wasmtime::component::Linker`].
pub struct InitContext<'a, T, U: Factor> {
    pub(crate) linker: &'a mut Linker<T>,
    pub(crate) get_data: GetDataFn<T, U>,
    pub(crate) get_data_with_table: GetDataWithTableFn<T, U>,
}

impl<'a, T, U: Factor> InitContext<'a, T, U> {
    #[doc(hidden)]
    pub fn new(
        linker: &'a mut Linker<T>,
        get_data: GetDataFn<T, U>,
        get_data_with_table: GetDataWithTableFn<T, U>,
    ) -> Self {
        Self {
            linker,
            get_data,
            get_data_with_table,
        }
    }

    /// Returns a mutable reference to the [`wasmtime::component::Linker`].
    pub fn linker(&mut self) -> &mut Linker<T> {
        self.linker
    }

    /// Returns a function that can be used to get the instance state for this factor.
    pub fn get_data_fn(&self) -> GetDataFn<T, U> {
        self.get_data
    }

    /// Returns a function that can be used to get the instance state for this
    /// factor along with the instance's [`ResourceTable`].
    pub fn get_data_with_table_fn(&self) -> GetDataWithTableFn<T, U> {
        self.get_data_with_table
    }

    /// Convenience method to link a binding to the linker.
    pub fn link_bindings(
        &mut self,
        add_to_linker: impl Fn(
            &mut Linker<T>,
            fn(&mut T) -> &mut FactorInstanceState<U>,
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
    pub fn new(
        app: &'a App,
        app_state: &'a T::AppState,
        runtime_config: Option<F::RuntimeConfig>,
    ) -> crate::Result<Self> {
        Ok(Self {
            app,
            app_state,
            runtime_config,
        })
    }

    /// Get the [`App`] being configured.
    pub fn app(&self) -> &'a App {
        self.app
    }

    /// Get the app state related to the given factor.
    pub fn app_state<U: Factor>(&self) -> crate::Result<&'a U::AppState> {
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
