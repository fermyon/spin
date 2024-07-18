use std::any::Any;

use spin_app::AppComponent;

use crate::{Error, Factor, RuntimeFactors};

/// A builder for a [`Factor`]'s per instance state.
pub trait FactorInstanceBuilder: Any {
    /// The per instance state of the factor.
    ///
    /// This is equivalent to the existing `HostComponent::Data` and ends up
    /// being stored in the `wasmtime::Store`. Any `bindgen` traits for this
    /// factor will be implemented on this type.
    type InstanceState: Send + 'static;

    /// Build the per instance state of the factor.
    fn build(self) -> anyhow::Result<Self::InstanceState>;
}

impl FactorInstanceBuilder for () {
    type InstanceState = ();

    fn build(self) -> anyhow::Result<Self::InstanceState> {
        Ok(())
    }
}

/// A helper trait for when the type implementing [`FactorInstanceBuilder`] is also the instance state.
pub trait SelfInstanceBuilder: Send + 'static {}

impl<T: SelfInstanceBuilder> FactorInstanceBuilder for T {
    type InstanceState = Self;

    fn build(self) -> anyhow::Result<Self::InstanceState> {
        Ok(self)
    }
}

/// A PrepareContext is passed to [`Factor::prepare`].
///
/// This gives the factor access to app state and the app component.
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

    /// Get the app state related to the factor.
    pub fn app_state(&self) -> &F::AppState {
        self.app_state
    }

    /// Get the app component.
    pub fn app_component(&self) -> &AppComponent {
        self.app_component
    }
}

/// The collection of all the already prepared `InstanceBuilder`s.
///
/// Use `InstanceBuilders::get_mut` to get a mutable reference to a specific factor's instance builder.
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
    pub fn get_mut<U: Factor>(&mut self) -> crate::Result<&mut U::InstanceBuilder> {
        T::instance_builder_mut::<U>(self.inner)
            .ok_or(Error::no_such_factor::<U>())?
            .ok_or_else(|| {
                Error::DependencyOrderingError(format!(
                    "{factor} builder requested before it was prepared",
                    factor = std::any::type_name::<U>()
                ))
            })
    }
}
