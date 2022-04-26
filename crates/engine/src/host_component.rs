use std::{any::Any, marker::PhantomData};

use spin_manifest::CoreComponent;
use wasmtime::Linker;

use crate::RuntimeContext;

/// Represents a host implementation of a Wasm interface.
pub trait HostComponent: Send + Sync {
    /// Host component runtime state.
    type Data: Any + Send;

    /// Add this component to the given Linker, using the given runtime state-getting closure.
    fn add_to_linker<T>(
        linker: &mut Linker<RuntimeContext<T>>,
        data_handle: HostComponentsDataHandle<Self::Data>,
    ) -> anyhow::Result<()>;

    /// Build a new runtime state object for the given component.
    fn build_data(&self, component: &CoreComponent) -> anyhow::Result<Self::Data>;
}
type HostComponentData = Box<dyn Any + Send>;

type DataBuilder = Box<dyn Fn(&CoreComponent) -> anyhow::Result<HostComponentData> + Send + Sync>;

#[derive(Default)]
pub(crate) struct HostComponents {
    data_builders: Vec<DataBuilder>,
}

impl HostComponents {
    pub(crate) fn insert<'a, T: 'static, Component: HostComponent + 'static>(
        &mut self,
        linker: &'a mut Linker<RuntimeContext<T>>,
        host_component: Component,
    ) -> anyhow::Result<()> {
        let handle = HostComponentsDataHandle {
            idx: self.data_builders.len(),
            _phantom: PhantomData,
        };
        Component::add_to_linker(linker, handle)?;
        self.data_builders.push(Box::new(move |c| {
            Ok(Box::new(host_component.build_data(c)?))
        }));
        Ok(())
    }

    pub(crate) fn build_data(&self, c: &CoreComponent) -> anyhow::Result<HostComponentsData> {
        Ok(HostComponentsData(
            self.data_builders
                .iter()
                .map(|build_data| build_data(c))
                .collect::<anyhow::Result<_>>()?,
        ))
    }
}

/// A collection of host component data.
#[derive(Default)]
pub struct HostComponentsData(Vec<HostComponentData>);

/// A handle to component data, used in HostComponent::add_to_linker.
pub struct HostComponentsDataHandle<T> {
    idx: usize,
    _phantom: PhantomData<fn(T) -> T>,
}

impl<T: 'static> HostComponentsDataHandle<T> {
    /// Get the component data associated with this handle from the RuntimeContext.
    pub fn get_mut<'a, U>(&self, ctx: &'a mut RuntimeContext<U>) -> &'a mut T {
        ctx.host_components_data
            .0
            .get_mut(self.idx)
            .unwrap()
            .downcast_mut()
            .unwrap()
    }
}

impl<T> Clone for HostComponentsDataHandle<T> {
    fn clone(&self) -> Self {
        Self {
            idx: self.idx,
            _phantom: PhantomData,
        }
    }
}

impl<T> Copy for HostComponentsDataHandle<T> {}
