use anyhow::Context;

use crate::{AppComponent, Factor, RuntimeFactors};

pub trait FactorInstancePreparer<F: Factor>: Sized {
    /// Returns a new instance of this preparer for the given [`Factor`].
    fn new<T: RuntimeFactors>(
        ctx: PrepareContext<F>,
        _preparers: InstancePreparers<T>,
    ) -> anyhow::Result<Self>;

    /// Returns a new instance of the associated [`Factor::InstanceState`].
    fn prepare(self) -> anyhow::Result<F::InstanceState>;
}

impl<F: Factor> FactorInstancePreparer<F> for ()
where
    F::InstanceState: Default,
{
    fn new<T: RuntimeFactors>(
        _ctx: PrepareContext<F>,
        _preparers: InstancePreparers<T>,
    ) -> anyhow::Result<Self> {
        Ok(())
    }

    fn prepare(self) -> anyhow::Result<F::InstanceState> {
        Ok(Default::default())
    }
}

/// A PrepareContext is passed to [`FactorInstancePreparer::new`], giving access
/// to any already-initialized [`FactorInstancePreparer`]s, allowing for
/// inter-[`Factor`] dependencies.
pub struct PrepareContext<'a, F: Factor> {
    pub(crate) factor: &'a F,
    pub(crate) app_config: &'a F::AppState,
    pub(crate) app_component: &'a AppComponent<'a>,
}

impl<'a, F: Factor> PrepareContext<'a, F> {
    #[doc(hidden)]
    pub fn new(
        factor: &'a F,
        app_config: &'a F::AppState,
        app_component: &'a AppComponent,
    ) -> Self {
        Self {
            factor,
            app_config,
            app_component,
        }
    }

    pub fn factor(&self) -> &F {
        self.factor
    }

    pub fn app_config(&self) -> &F::AppState {
        self.app_config
    }

    pub fn app_component(&self) -> &AppComponent {
        self.app_component
    }
}

pub struct InstancePreparers<'a, T: RuntimeFactors> {
    pub(crate) inner: &'a mut T::InstancePreparers,
}

impl<'a, T: RuntimeFactors> InstancePreparers<'a, T> {
    #[doc(hidden)]
    pub fn new(inner: &'a mut T::InstancePreparers) -> Self {
        Self { inner }
    }

    /// Returns a already-initialized preparer for the given [`Factor`].
    ///
    /// Fails if the current [`RuntimeFactors`] does not include the given
    /// [`Factor`] or if the given [`Factor`]'s preparer has not been
    /// initialized yet (because it is sequenced after this factor).
    pub fn get_mut<F: Factor>(&mut self) -> crate::Result<&mut F::InstancePreparer> {
        T::instance_preparer_mut::<F>(self.inner)?.context("preparer not initialized")
    }
}
