use std::{any::Any, sync::Arc};

use anyhow::{Context, Result};
use spin_core::{AnyHostComponentDataHandle, EngineBuilder, HostComponent, HostComponentsData};

use crate::AppComponent;

/// A trait for "dynamic" Spin host components.
///
/// This extends [`HostComponent`] to support per-[`AppComponent`] dynamic
/// runtime configuration.
pub trait DynamicHostComponent: HostComponent {
    /// Called on [`AppComponent`] instance initialization.
    ///
    /// The `data` returned by [`HostComponent::build_data`] is passed, along
    /// with a reference to the `component` being instantiated.
    fn update_data(&self, data: &mut Self::Data, component: &AppComponent) -> Result<()>;
}

impl<DHC: DynamicHostComponent> DynamicHostComponent for Arc<DHC> {
    fn update_data(&self, data: &mut Self::Data, component: &AppComponent) -> Result<()> {
        (**self).update_data(data, component)
    }
}

trait DynSafeDynamicHostComponent {
    fn update_data_any(&self, data: &mut dyn Any, component: &AppComponent) -> Result<()>;
}

impl<T: DynamicHostComponent> DynSafeDynamicHostComponent for T
where
    T::Data: Any,
{
    fn update_data_any(&self, data: &mut dyn Any, component: &AppComponent) -> Result<()> {
        let data = data.downcast_mut().context("wrong data type")?;
        self.update_data(data, component)
    }
}

type ArcDynamicHostComponent = Arc<dyn DynSafeDynamicHostComponent + Send + Sync>;

struct DynamicHostComponentWithHandle {
    host_component: ArcDynamicHostComponent,
    handle: AnyHostComponentDataHandle,
}

#[derive(Default)]
pub struct DynamicHostComponents {
    host_components: Vec<DynamicHostComponentWithHandle>,
}

impl DynamicHostComponents {
    pub fn add_dynamic_host_component<T: Send + Sync, DHC: DynamicHostComponent>(
        &mut self,
        engine_builder: &mut EngineBuilder<T>,
        host_component: DHC,
    ) -> Result<()> {
        let host_component = Arc::new(host_component);
        let handle = engine_builder
            .add_host_component(host_component.clone())?
            .into();
        self.host_components.push(DynamicHostComponentWithHandle {
            host_component,
            handle,
        });
        Ok(())
    }

    pub fn update_data(
        &self,
        host_components_data: &mut HostComponentsData,
        component: &AppComponent,
    ) -> Result<()> {
        for DynamicHostComponentWithHandle {
            host_component,
            handle,
        } in &self.host_components
        {
            let data = host_components_data.get_or_insert_any(*handle);
            host_component.update_data_any(data, component)?;
        }
        Ok(())
    }
}
