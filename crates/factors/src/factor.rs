use std::any::Any;

use anyhow::Context;

use crate::{App, FactorInstancePreparer, Linker, ModuleLinker, RuntimeConfig, RuntimeFactors};

pub trait Factor: Any + Sized {
    /// Per-app state for this factor.
    ///
    /// See [`Factor::configure_app`].
    type AppState: Default;

    /// The [`FactorInstancePreparer`] for this factor.
    type InstancePreparer: FactorInstancePreparer<Self>;

    /// The per-instance state for this factor, constructed by a
    /// [`FactorInstancePreparer`] and available to any host-provided imports
    /// defined by this factor.
    type InstanceState;

    /// Initializes this Factor for a runtime. This will be called at most once,
    /// before any call to [`FactorInstancePreparer::new`]
    fn init<T: RuntimeFactors>(&mut self, mut ctx: InitContext<T, Self>) -> anyhow::Result<()> {
        // TODO: Should `ctx` always be immut? Rename this param/type?
        _ = &mut ctx;
        Ok(())
    }

    /// Performs factor-specific validation and configuration for the given
    /// [`App`]. A runtime may - but is not required to - reuse the returned
    /// config across multiple instances. Note that this may be called without
    /// any call to `init` in cases where only validation is needed.
    fn configure_app<T: RuntimeFactors>(
        &self,
        ctx: ConfigureAppContext<T>,
        _runtime_config: &mut impl RuntimeConfig,
    ) -> anyhow::Result<Self::AppState> {
        _ = ctx;
        Ok(Default::default())
    }
}

pub(crate) type GetDataFn<Facts, Fact> =
    fn(&mut <Facts as RuntimeFactors>::InstanceState) -> &mut <Fact as Factor>::InstanceState;

/// An InitContext is passed to [`Factor::init`], giving access to the global
/// common [`wasmtime::component::Linker`].
pub struct InitContext<'a, T: RuntimeFactors, F: Factor> {
    pub(crate) linker: Option<&'a mut Linker<T>>,
    pub(crate) module_linker: Option<&'a mut ModuleLinker<T>>,
    pub(crate) get_data: GetDataFn<T, F>,
}

impl<'a, T: RuntimeFactors, F: Factor> InitContext<'a, T, F> {
    #[doc(hidden)]
    pub fn new(
        linker: Option<&'a mut Linker<T>>,
        module_linker: Option<&'a mut ModuleLinker<T>>,
        get_data: GetDataFn<T, F>,
    ) -> Self {
        Self {
            linker,
            module_linker,
            get_data,
        }
    }

    pub fn linker(&mut self) -> Option<&mut Linker<T>> {
        self.linker.as_deref_mut()
    }

    pub fn module_linker(&mut self) -> Option<&mut ModuleLinker<T>> {
        self.module_linker.as_deref_mut()
    }

    pub fn get_data_fn(&self) -> GetDataFn<T, F> {
        self.get_data
    }

    pub fn link_bindings(
        &mut self,
        add_to_linker: impl Fn(
            &mut Linker<T>,
            fn(&mut T::InstanceState) -> &mut F::InstanceState,
        ) -> anyhow::Result<()>,
    ) -> anyhow::Result<()>
where {
        if let Some(linker) = self.linker.as_deref_mut() {
            add_to_linker(linker, self.get_data)
        } else {
            Ok(())
        }
    }

    pub fn link_module_bindings(
        &mut self,
        add_to_linker: impl Fn(
            &mut ModuleLinker<T>,
            fn(&mut T::InstanceState) -> &mut F::InstanceState,
        ) -> anyhow::Result<()>,
    ) -> anyhow::Result<()>
where {
        if let Some(linker) = self.module_linker.as_deref_mut() {
            add_to_linker(linker, self.get_data)
        } else {
            Ok(())
        }
    }
}

pub struct ConfigureAppContext<'a, T: RuntimeFactors> {
    pub(crate) app: &'a App,
    pub(crate) app_configs: &'a T::AppState,
}

impl<'a, T: RuntimeFactors> ConfigureAppContext<'a, T> {
    #[doc(hidden)]
    pub fn new(app: &'a App, app_configs: &'a T::AppState) -> Self {
        Self { app, app_configs }
    }

    pub fn app(&self) -> &App {
        self.app
    }

    pub fn app_config<F: Factor>(&self) -> crate::Result<&F::AppState> {
        T::app_config::<F>(self.app_configs).context("no such factor")
    }
}

pub struct ConfiguredApp<T: RuntimeFactors> {
    app: App,
    app_configs: T::AppState,
}

impl<T: RuntimeFactors> ConfiguredApp<T> {
    #[doc(hidden)]
    pub fn new(app: App, app_configs: T::AppState) -> Self {
        Self { app, app_configs }
    }

    pub fn app(&self) -> &App {
        &self.app
    }

    pub fn app_config<F: Factor>(&self) -> crate::Result<&F::AppState> {
        T::app_config::<F>(&self.app_configs).context("no such factor")
    }
}
