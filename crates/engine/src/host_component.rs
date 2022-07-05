use std::{any::Any, marker::PhantomData};

use anyhow::Result;
use spin_manifest::CoreComponent;
use wasmtime::Linker;

use crate::RuntimeContext;

/// Represents a host implementation of a Wasm interface.
pub trait HostComponent: Send + Sync {
    /// Host component runtime state.
    type State: Any + Send;

    /// Add this component to the given Linker, using the given runtime state-getting handle.
    fn add_to_linker<T>(
        linker: &mut Linker<RuntimeContext<T>>,
        state_handle: HostComponentsStateHandle<Self::State>,
    ) -> Result<()>;

    /// Build a new runtime state object for the given component.
    fn build_state(&self, component: &CoreComponent) -> Result<Self::State>;
}
type HostComponentState = Box<dyn Any + Send>;

type StateBuilder = Box<dyn Fn(&CoreComponent) -> Result<HostComponentState> + Send + Sync>;

#[derive(Default)]
pub(crate) struct HostComponents {
    state_builders: Vec<StateBuilder>,
}

impl HostComponents {
    pub(crate) fn insert<T: 'static, Component: HostComponent + 'static>(
        &mut self,
        linker: &mut Linker<RuntimeContext<T>>,
        host_component: Component,
    ) -> Result<()> {
        let handle = HostComponentsStateHandle {
            idx: self.state_builders.len(),
            _phantom: PhantomData,
        };
        Component::add_to_linker(linker, handle)?;
        self.state_builders.push(Box::new(move |c| {
            Ok(Box::new(host_component.build_state(c)?))
        }));
        Ok(())
    }

    pub(crate) fn build_state(&self, c: &CoreComponent) -> Result<HostComponentsState> {
        Ok(HostComponentsState(
            self.state_builders
                .iter()
                .map(|build_state| build_state(c))
                .collect::<Result<_>>()?,
        ))
    }
}

/// A collection of host components state.
#[derive(Default)]
pub struct HostComponentsState(Vec<HostComponentState>);

/// A handle to component state, used in HostComponent::add_to_linker.
pub struct HostComponentsStateHandle<T> {
    idx: usize,
    _phantom: PhantomData<fn(T) -> T>,
}

impl<T: 'static> HostComponentsStateHandle<T> {
    /// Get a ref to the component state associated with this handle from the RuntimeContext.
    pub fn get<'a, U>(&self, ctx: &'a RuntimeContext<U>) -> &'a T {
        ctx.host_components_state
            .0
            .get(self.idx)
            .unwrap()
            .downcast_ref()
            .unwrap()
    }

    /// Get a mutable ref to the component state associated with this handle from the RuntimeContext.
    pub fn get_mut<'a, U>(&self, ctx: &'a mut RuntimeContext<U>) -> &'a mut T {
        ctx.host_components_state
            .0
            .get_mut(self.idx)
            .unwrap()
            .downcast_mut()
            .unwrap()
    }
}

impl<T> Clone for HostComponentsStateHandle<T> {
    fn clone(&self) -> Self {
        Self {
            idx: self.idx,
            _phantom: PhantomData,
        }
    }
}

impl<T> Copy for HostComponentsStateHandle<T> {}
