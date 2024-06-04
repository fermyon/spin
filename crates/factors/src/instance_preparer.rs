use anyhow::Context;

use crate::{AppComponent, Factor, SpinFactors};

pub trait FactorInstancePreparer<T: Factor>: Sized {
    /// Returns a new instance of this preparer for the given [`Factor`].
    fn new<Factors: SpinFactors>(
        ctx: PrepareContext<T>,
        _preparers: InstancePreparers<Factors>,
    ) -> anyhow::Result<Self>;

    /// Returns a new instance of the associated [`Factor::InstanceState`].
    fn prepare(self) -> anyhow::Result<T::InstanceState>;
}

impl<T: Factor> FactorInstancePreparer<T> for ()
where
    T::InstanceState: Default,
{
    fn new<Factors: SpinFactors>(
        _ctx: PrepareContext<T>,
        _preparers: InstancePreparers<Factors>,
    ) -> anyhow::Result<Self> {
        Ok(())
    }

    fn prepare(self) -> anyhow::Result<T::InstanceState> {
        Ok(Default::default())
    }
}

/// A PrepareContext is passed to [`FactorInstancePreparer::new`], giving access
/// to any already-initialized [`FactorInstancePreparer`]s, allowing for
/// inter-[`Factor`] dependencies.
pub struct PrepareContext<'a, T: Factor> {
    pub(crate) factor: &'a T,
    pub(crate) app_config: &'a T::AppConfig,
    pub(crate) app_component: &'a AppComponent<'a>,
}

impl<'a, T: Factor> PrepareContext<'a, T> {
    #[doc(hidden)]
    pub fn new(
        factor: &'a T,
        app_config: &'a T::AppConfig,
        app_component: &'a AppComponent,
    ) -> Self {
        Self {
            factor,
            app_config,
            app_component,
        }
    }

    pub fn factor(&self) -> &T {
        self.factor
    }

    pub fn app_config(&self) -> &T::AppConfig {
        self.app_config
    }

    pub fn app_component(&self) -> &AppComponent {
        self.app_component
    }
}

pub struct InstancePreparers<'a, Factors: SpinFactors> {
    pub(crate) inner: &'a mut Factors::InstancePreparers,
}

impl<'a, Factors: SpinFactors> InstancePreparers<'a, Factors> {
    #[doc(hidden)]
    pub fn new(inner: &'a mut Factors::InstancePreparers) -> Self {
        Self { inner }
    }

    /// Returns a already-initialized preparer for the given [`Factor`].
    ///
    /// Fails if the current [`SpinFactors`] does not include the given
    /// [`Factor`] or if the given [`Factor`]'s preparer has not been
    /// initialized yet (because it is sequenced after this factor).
    pub fn get_mut<T: Factor>(&mut self) -> crate::Result<&mut T::InstancePreparer> {
        Factors::instance_preparer_mut::<T>(self.inner)?.context("preparer not initialized")
    }
}
