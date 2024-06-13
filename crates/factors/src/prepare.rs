use std::any::Any;

use anyhow::Context;

use crate::{AppComponent, Factor, RuntimeFactors};

pub trait FactorInstanceBuilder: Any {
    type InstanceState: Send + 'static;

    fn build(self) -> anyhow::Result<Self::InstanceState>;
}

impl FactorInstanceBuilder for () {
    type InstanceState = ();

    fn build(self) -> anyhow::Result<Self::InstanceState> {
        Ok(())
    }
}

pub trait SelfInstanceBuilder: Send + 'static {}

impl<T: SelfInstanceBuilder> FactorInstanceBuilder for T {
    type InstanceState = Self;

    fn build(self) -> anyhow::Result<Self::InstanceState> {
        Ok(self)
    }
}

/// A PrepareContext is passed to [`Factor::prepare`], giving access to any
/// already-initialized [`FactorInstanceBuilder`]s, allowing for
/// inter-[`Factor`] dependencies.
pub struct PrepareContext<'a, F: Factor> {
    pub(crate) app_state: &'a F::AppState,
    pub(crate) app_component: &'a AppComponent<'a>,
}

impl<'a, F: Factor> PrepareContext<'a, F> {
    #[doc(hidden)]
    pub fn new(app_state: &'a F::AppState, app_component: &'a AppComponent) -> Self {
        Self {
            app_state,
            app_component,
        }
    }

    pub fn app_state(&self) -> &F::AppState {
        self.app_state
    }

    pub fn app_component(&self) -> &AppComponent {
        self.app_component
    }
}

pub struct InstanceBuilders<'a, T: RuntimeFactors> {
    pub(crate) inner: &'a mut T::InstanceBuilders,
}

impl<'a, T: RuntimeFactors> InstanceBuilders<'a, T> {
    #[doc(hidden)]
    pub fn new(inner: &'a mut T::InstanceBuilders) -> Self {
        Self { inner }
    }

    /// Returns the prepared [`FactorInstanceBuilder`] for the given [`Factor`].
    ///
    /// Fails if the current [`RuntimeFactors`] does not include the given
    /// [`Factor`] or if the given [`Factor`]'s builder has not been prepared
    /// yet (because it is sequenced after this factor).
    pub fn get_mut<F: Factor>(&mut self) -> crate::Result<&mut F::InstanceBuilder> {
        T::instance_builder_mut::<F>(self.inner)
            .context("no such factor")?
            .context("builder not prepared")
    }
}
