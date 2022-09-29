use std::sync::Arc;

use spin_core::{EngineBuilder, HostComponent, HostComponentsData};

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
    fn update_data(&self, data: &mut Self::Data, component: &AppComponent) -> anyhow::Result<()>;
}

impl<DHC: DynamicHostComponent> DynamicHostComponent for Arc<DHC> {
    fn update_data(&self, data: &mut Self::Data, component: &AppComponent) -> anyhow::Result<()> {
        (**self).update_data(data, component)
    }
}

type DataUpdater =
    Box<dyn Fn(&mut HostComponentsData, &AppComponent) -> anyhow::Result<()> + Send + Sync>;

#[derive(Default)]
pub struct DynamicHostComponents {
    data_updaters: Vec<DataUpdater>,
}

impl DynamicHostComponents {
    pub fn add_dynamic_host_component<T: Send + Sync, DHC: DynamicHostComponent>(
        &mut self,
        engine_builder: &mut EngineBuilder<T>,
        host_component: DHC,
    ) -> anyhow::Result<()> {
        let host_component = Arc::new(host_component);
        let handle = engine_builder.add_host_component(host_component.clone())?;
        self.data_updaters
            .push(Box::new(move |host_components_data, component| {
                let data = host_components_data.get_or_insert(handle);
                host_component.update_data(data, component)
            }));
        Ok(())
    }

    pub fn update_data(
        &self,
        host_components_data: &mut HostComponentsData,
        component: &AppComponent,
    ) -> anyhow::Result<()> {
        for data_updater in &self.data_updaters {
            data_updater(host_components_data, component)?;
        }
        Ok(())
    }
}
