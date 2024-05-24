use std::{any::Any, marker::PhantomData};

use anyhow::Context;
pub use spin_factors_derive::SpinFactors;

pub use wasmtime;

pub type Error = wasmtime::Error;
pub type Result<T, E = Error> = std::result::Result<T, E>;

pub type Linker<Factors> = wasmtime::component::Linker<<Factors as SpinFactors>::InstanceState>;
pub type ModuleLinker<Factors> = wasmtime::Linker<<Factors as SpinFactors>::InstanceState>;

// Temporary wrappers while refactoring
pub type App = spin_app::App<'static, spin_app::InertLoader>;
pub type AppComponent<'a> = spin_app::AppComponent<'a, spin_app::InertLoader>;

pub trait Factor: Any + Sized {
    /// App configuration for this factor.
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
    fn init<Factors: SpinFactors>(&mut self, mut ctx: InitContext<Factors, Self>) -> Result<()> {
        _ = &mut ctx;
        Ok(())
    }

    /// Performs factor-specific validation and configuration for the given
    /// [`App`] and [`RuntimeConfig`]. A runtime may - but is not required to -
    /// reuse the returned config across multiple instances. Note that this may
    /// be called without any call to `init` in cases where only validation is
    /// needed.
    fn configure_app<Factors: SpinFactors>(
        &self,
        app: &App,
        _ctx: ConfigureAppContext<Factors>,
    ) -> Result<Self::AppConfig> {
        _ = app;
        Ok(Default::default())
    }
}

type GetDataFn<Factors, Fact> =
    fn(&mut <Factors as SpinFactors>::InstanceState) -> &mut <Fact as Factor>::InstanceState;

/// An InitContext is passed to [`Factor::init`], giving access to the global
/// common [`wasmtime::component::Linker`].
pub struct InitContext<'a, Factors: SpinFactors, Fact: Factor> {
    linker: Option<&'a mut Linker<Factors>>,
    module_linker: Option<&'a mut ModuleLinker<Factors>>,
    get_data: GetDataFn<Factors, Fact>,
}

impl<'a, Factors: SpinFactors, Fact: Factor> InitContext<'a, Factors, Fact> {
    #[doc(hidden)]
    pub fn new(
        linker: Option<&'a mut Linker<Factors>>,
        module_linker: Option<&'a mut ModuleLinker<Factors>>,
        get_data: GetDataFn<Factors, Fact>,
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

    pub fn get_data_fn(&self) -> GetDataFn<Factors, Fact> {
        self.get_data
    }

    pub fn link_bindings(
        &mut self,
        add_to_linker: impl Fn(
            &mut Linker<Factors>,
            fn(&mut Factors::InstanceState) -> &mut Fact::InstanceState,
        ) -> Result<()>,
    ) -> Result<()>
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
            fn(&mut Factors::InstanceState) -> &mut Fact::InstanceState,
        ) -> Result<()>,
    ) -> Result<()>
where {
        if let Some(linker) = self.module_linker.as_deref_mut() {
            add_to_linker(linker, self.get_data)
        } else {
            Ok(())
        }
    }
}

pub struct ConfigureAppContext<'a, Factors: SpinFactors> {
    app_configs: &'a Factors::AppConfigs,
}

impl<'a, Factors: SpinFactors> ConfigureAppContext<'a, Factors> {
    #[doc(hidden)]
    pub fn new(app_configs: &'a Factors::AppConfigs) -> Self {
        Self { app_configs }
    }

    pub fn app_config<T: Factor>(&self) -> Result<&T::AppConfig> {
        Factors::app_config::<T>(self.app_configs).context("no such factor")
    }
}

pub trait FactorInstancePreparer<T: Factor>: Sized {
    /// Returns a new instance of this preparer for the given [`Factor`].
    fn new<Factors: SpinFactors>(
        factor: &T,
        app_component: &AppComponent,
        _ctx: PrepareContext<Factors>,
    ) -> Result<Self>;

    /// Returns a new instance of the associated [`Factor::InstanceState`].
    fn prepare(self) -> Result<T::InstanceState>;
}

/// A PrepareContext is passed to [`FactorInstancePreparer::new`], giving access
/// to any already-initialized [`FactorInstancePreparer`]s, allowing for
/// inter-[`Factor`] dependencies.
pub struct PrepareContext<'a, Factors: SpinFactors> {
    configured_app: &'a ConfiguredApp<Factors>,
    instance_preparers: &'a mut Factors::InstancePreparers,
}

impl<'a, Factors: SpinFactors> PrepareContext<'a, Factors> {
    #[doc(hidden)]
    pub fn new(
        configured_app: &'a ConfiguredApp<Factors>,
        instance_preparers: &'a mut Factors::InstancePreparers,
    ) -> Self {
        Self {
            configured_app,
            instance_preparers,
        }
    }

    pub fn app_config<T: Factor>(&self) -> Result<&T::AppConfig> {
        self.configured_app.app_config::<T>()
    }

    /// Returns a already-initialized preparer for the given [`Factor`].
    ///
    /// Fails if the current [`SpinFactors`] does not include the given
    /// [`Factor`] or if the given [`Factor`]'s preparer has not been
    /// initialized yet (because it is sequenced after this factor).
    pub fn instance_preparer_mut<T: Factor>(&mut self) -> Result<&mut T::InstancePreparer> {
        Factors::instance_preparer_mut::<T>(self.instance_preparers)
            .and_then(|maybe_preparer| maybe_preparer.context("preparer not yet initialized"))
            .with_context(|| {
                format!(
                    "could not get instance preparer for {}",
                    std::any::type_name::<T>()
                )
            })
    }
}

impl<T: Factor> FactorInstancePreparer<T> for ()
where
    T::InstanceState: Default,
{
    fn new<Factors: SpinFactors>(
        factor: &T,
        app_component: &AppComponent,
        _ctx: PrepareContext<Factors>,
    ) -> Result<Self> {
        (_, _) = (factor, app_component);
        Ok(())
    }

    fn prepare(self) -> Result<T::InstanceState> {
        Ok(Default::default())
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

    pub fn app_config<T: Factor>(&self) -> Result<&T::AppConfig> {
        Factors::app_config::<T>(&self.app_configs).context("no such factor")
    }
}

/// Implemented by `#[derive(SpinFactors)]`
pub trait SpinFactors: Sized {
    type AppConfigs;
    type InstancePreparers;
    type InstanceState: Send + 'static;

    #[doc(hidden)]
    unsafe fn instance_preparer_offset<T: Factor>() -> Option<usize>;

    #[doc(hidden)]
    unsafe fn instance_state_offset<T: Factor>() -> Option<usize>;

    fn app_config<T: Factor>(app_configs: &Self::AppConfigs) -> Option<&T::AppConfig>;

    fn instance_state_getter<T: Factor>() -> Option<Getter<Self::InstanceState, T::InstanceState>> {
        let offset = unsafe { Self::instance_state_offset::<T>()? };
        Some(Getter {
            offset,
            _phantom: PhantomData,
        })
    }

    fn instance_state_getter2<T1: Factor, T2: Factor>(
    ) -> Option<Getter2<Self::InstanceState, T1::InstanceState, T2::InstanceState>> {
        let offset1 = unsafe { Self::instance_state_offset::<T1>()? };
        let offset2 = unsafe { Self::instance_state_offset::<T2>()? };
        assert_ne!(
            offset1, offset2,
            "instance_state_getter2 with same factor twice would alias"
        );
        Some(Getter2 {
            offset1,
            offset2,
            _phantom: PhantomData,
        })
    }

    fn instance_preparer_mut<T: Factor>(
        preparers: &mut Self::InstancePreparers,
    ) -> Result<Option<&mut T::InstancePreparer>> {
        unsafe {
            let offset = Self::instance_preparer_offset::<T>().context("no such factor")?;
            let ptr = preparers as *mut Self::InstancePreparers;
            let opt = &mut *ptr.add(offset).cast::<Option<T::InstancePreparer>>();
            Ok(opt.as_mut())
        }
    }
}

pub struct Getter<T, U> {
    offset: usize,
    _phantom: PhantomData<fn(&mut T) -> &mut U>,
}

impl<T, U> Getter<T, U> {
    pub fn get_mut<'a>(&self, container: &'a mut T) -> &'a mut U {
        let ptr = container as *mut T;
        unsafe { &mut *ptr.add(self.offset).cast::<U>() }
    }
}

impl<T, U> Clone for Getter<T, U> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T, U> Copy for Getter<T, U> {}

pub struct Getter2<T, U, V> {
    offset1: usize,
    offset2: usize,
    #[allow(clippy::type_complexity)]
    _phantom: PhantomData<fn(&mut T) -> (&mut U, &mut V)>,
}

impl<T, U, V> Getter2<T, U, V> {
    pub fn get_mut<'a>(&self, container: &'a mut T) -> (&'a mut U, &'a mut V)
    where
        T: 'static,
        U: 'static,
        V: 'static,
    {
        let ptr = container as *mut T;
        unsafe {
            (
                &mut *ptr.add(self.offset1).cast::<U>(),
                &mut *ptr.add(self.offset2).cast::<V>(),
            )
        }
    }
}

impl<T, U, V> Clone for Getter2<T, U, V> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T, U, V> Copy for Getter2<T, U, V> {}
