use std::any::Any;

use anyhow::Context;

use crate::{App, FactorInstancePreparer, Linker, ModuleLinker, RuntimeConfig, SpinFactors};

pub trait Factor: Any + Sized {
    /// Per-app configuration for this factor.
    ///
    /// See [`Factor::configure_app`].
    type AppConfig: Default;

    /// The [`FactorInstancePreparer`] for this factor.
    type InstancePreparer: FactorInstancePreparer<Self>;

    /// The per-instance state for this factor, constructed by a
    /// [`FactorInstancePreparer`] and available to any host-provided imports
    /// defined by this factor.
    type InstanceState;

    /// Initializes this Factor for a runtime. This will be called at most once,
    /// before any call to [`FactorInstancePreparer::new`]
    fn init<Factors: SpinFactors>(
        &mut self,
        mut ctx: InitContext<Factors, Self>,
    ) -> anyhow::Result<()> {
        // TODO: Should `ctx` always be immut? Rename this param/type?
        _ = &mut ctx;
        Ok(())
    }

    /// Performs factor-specific validation and configuration for the given
    /// [`App`]. A runtime may - but is not required to - reuse the returned
    /// config across multiple instances. Note that this may be called without
    /// any call to `init` in cases where only validation is needed.
    fn configure_app<Factors: SpinFactors>(
        &self,
        ctx: ConfigureAppContext<Factors>,
        _runtime_config: &mut impl RuntimeConfig,
    ) -> anyhow::Result<Self::AppConfig> {
        _ = ctx;
        Ok(Default::default())
    }
}

pub(crate) type GetDataFn<Factors, Fact> =
    fn(&mut <Factors as SpinFactors>::InstanceState) -> &mut <Fact as Factor>::InstanceState;

/// An InitContext is passed to [`Factor::init`], giving access to the global
/// common [`wasmtime::component::Linker`].
pub struct InitContext<'a, Factors: SpinFactors, T: Factor> {
    pub(crate) linker: Option<&'a mut Linker<Factors>>,
    pub(crate) module_linker: Option<&'a mut ModuleLinker<Factors>>,
    pub(crate) get_data: GetDataFn<Factors, T>,
}

impl<'a, Factors: SpinFactors, T: Factor> InitContext<'a, Factors, T> {
    #[doc(hidden)]
    pub fn new(
        linker: Option<&'a mut Linker<Factors>>,
        module_linker: Option<&'a mut ModuleLinker<Factors>>,
        get_data: GetDataFn<Factors, T>,
    ) -> Self {
        Self {
            linker,
            module_linker,
            get_data,
        }
    }

    pub fn linker(&mut self) -> Option<&mut Linker<Factors>> {
        self.linker.as_deref_mut()
    }

    pub fn module_linker(&mut self) -> Option<&mut ModuleLinker<Factors>> {
        self.module_linker.as_deref_mut()
    }

    pub fn get_data_fn(&self) -> GetDataFn<Factors, T> {
        self.get_data
    }

    pub fn link_bindings(
        &mut self,
        add_to_linker: impl Fn(
            &mut Linker<Factors>,
            fn(&mut Factors::InstanceState) -> &mut T::InstanceState,
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
            &mut ModuleLinker<Factors>,
            fn(&mut Factors::InstanceState) -> &mut T::InstanceState,
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

pub struct ConfigureAppContext<'a, Factors: SpinFactors> {
    pub(crate) app: &'a App,
    pub(crate) app_configs: &'a Factors::AppConfigs,
}

impl<'a, Factors: SpinFactors> ConfigureAppContext<'a, Factors> {
    #[doc(hidden)]
    pub fn new(app: &'a App, app_configs: &'a Factors::AppConfigs) -> Self {
        Self { app, app_configs }
    }

    pub fn app(&self) -> &App {
        self.app
    }

    pub fn app_config<T: Factor>(&self) -> crate::Result<&T::AppConfig> {
        Factors::app_config::<T>(self.app_configs).context("no such factor")
    }
}

pub struct ConfiguredApp<Factors: SpinFactors> {
    app: App,
    app_configs: Factors::AppConfigs,
}

impl<Factors: SpinFactors> ConfiguredApp<Factors> {
    #[doc(hidden)]
    pub fn new(app: App, app_configs: Factors::AppConfigs) -> Self {
        Self { app, app_configs }
    }

    pub fn app(&self) -> &App {
        &self.app
    }

    pub fn app_config<T: Factor>(&self) -> crate::Result<&T::AppConfig> {
        Factors::app_config::<T>(&self.app_configs).context("no such factor")
    }
}
