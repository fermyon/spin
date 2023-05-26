use std::{any::Any, sync::Arc};

use anyhow::Context;
use spin_core::{
    AnyHostComponentDataHandle, EngineBuilder, HostComponent, HostComponentDataHandle,
    HostComponentsData,
};

use crate::{App, AppComponent};

/// A trait for "dynamic" Spin host components.
///
/// This extends [`HostComponent`] to support per-[`AppComponent`] dynamic
/// runtime configuration.
pub trait DynamicHostComponent: HostComponent {
    /// Called on [`AppComponent`] instance initialization.
    ///
    /// The `data` returned by [`HostComponent::build_data`] is passed, along
    /// with a reference to the `component` being instantiated.
    fn update_data(&self, data: &mut Self::Data, component: &AppComponent) -> anyhow::Result<()>;

    /// Called on [`App`] load to validate any configuration needed by this
    /// host component.
    ///
    /// Note that the _absence_ of configuration should not be treated as an
    /// error here, as the app may not use this host component at all.
    #[allow(unused_variables)]
    fn validate_app(&self, app: &App) -> anyhow::Result<()> {
        Ok(())
    }
}

impl<DHC: DynamicHostComponent> DynamicHostComponent for Arc<DHC> {
    fn update_data(&self, data: &mut Self::Data, component: &AppComponent) -> anyhow::Result<()> {
        (**self).update_data(data, component)
    }
}

type AnyData = Box<dyn Any + Send>;

trait DynSafeDynamicHostComponent {
    fn update_data_any(&self, data: &mut AnyData, component: &AppComponent) -> anyhow::Result<()>;
    fn validate_app(&self, app: &App) -> anyhow::Result<()>;
}

impl<T: DynamicHostComponent> DynSafeDynamicHostComponent for T
where
    T::Data: Any,
{
    fn update_data_any(&self, data: &mut AnyData, component: &AppComponent) -> anyhow::Result<()> {
        let data = data.downcast_mut().context("wrong data type")?;
        self.update_data(data, component)
    }

    fn validate_app(&self, app: &App) -> anyhow::Result<()> {
        T::validate_app(self, app)
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
    ) -> anyhow::Result<HostComponentDataHandle<DHC>> {
        let host_component = Arc::new(host_component);
        let handle = engine_builder.add_host_component(host_component.clone())?;
        self.host_components.push(DynamicHostComponentWithHandle {
            host_component,
            handle: handle.into(),
        });
        Ok(handle.into())
    }

    pub fn update_data(
        &self,
        host_components_data: &mut HostComponentsData,
        component: &AppComponent,
    ) -> anyhow::Result<()> {
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

    pub fn validate_app(&self, app: &App) -> anyhow::Result<()> {
        for DynamicHostComponentWithHandle { host_component, .. } in &self.host_components {
            host_component.validate_app(app)?;
        }
        Ok(())
    }
}
