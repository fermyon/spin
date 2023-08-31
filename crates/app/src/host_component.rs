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
///
/// Dynamic host components differ from regular host components in that they can be
/// configured on a per-component basis.
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

/// A version of `DynamicHostComponent` which can be made into a trait object.
///
/// This is only implemented for `T: DynamicHostComponent`. We want to make `DynamicHostComponent`
/// into a trait object so that we can store them into a heterogeneous collection in `DynamicHostComponents`.
///
/// `DynamicHostComponent` can't be made into a trait object itself since `HostComponent::add_to_linker`
/// does not have a `self` parameter (and thus cannot be add to the object's vtable).
trait DynSafeDynamicHostComponent {
    /// The moral equivalent to `DynamicHostComponent::update_data`
    fn update_data_any(&self, data: &mut dyn Any, component: &AppComponent) -> anyhow::Result<()>;
    /// The moral equivalent to `DynamicHostComponent::validate_app`
    fn validate_app(&self, app: &App) -> anyhow::Result<()>;
}

impl<T: DynamicHostComponent> DynSafeDynamicHostComponent for T
where
    T::Data: Any,
{
    fn update_data_any(&self, data: &mut dyn Any, component: &AppComponent) -> anyhow::Result<()> {
        let data = data.downcast_mut().context("wrong data type")?;
        self.update_data(data, component)
    }

    fn validate_app(&self, app: &App) -> anyhow::Result<()> {
        T::validate_app(self, app)
    }
}

struct DynamicHostComponentWithHandle {
    host_component: Arc<dyn DynSafeDynamicHostComponent + Send + Sync>,
    handle: AnyHostComponentDataHandle,
}

/// A heterogeneous collection of dynamic host components.
///
/// This is stored in an `AppLoader` so that the host components
/// can be referenced and updated at a later point. This is effectively
/// what makes a `DynamicHostComponent` "dynamic" and differentiates it from
/// a regular `HostComponent`.
#[derive(Default)]
pub(crate) struct DynamicHostComponents {
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
            host_component.update_data_any(data.as_mut(), component)?;
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
