use crate::{KeyValueDispatch, StoreManager};
use spin_app::{AppComponent, DynamicHostComponent};
use spin_core::HostComponent;
use std::sync::Arc;

pub trait StoreManagerManager: Sync + Send {
    fn get(&self, component: &AppComponent) -> Arc<dyn StoreManager>;
}

impl<F: (Fn(&AppComponent) -> Arc<dyn StoreManager>) + Sync + Send> StoreManagerManager for F {
    fn get(&self, component: &AppComponent) -> Arc<dyn StoreManager> {
        self(component)
    }
}

/// Help the rustc type inference engine understand that the specified closure has a higher-order bound so it can
/// be used as a [`StoreManagerManager`].
///
/// See https://stackoverflow.com/a/46198877 for details.
pub fn manager<F: for<'a> Fn(&'a AppComponent) -> Arc<dyn StoreManager>>(f: F) -> F {
    f
}

pub struct KeyValueComponent {
    capacity: u32,
    manager: Box<dyn StoreManagerManager>,
}

impl KeyValueComponent {
    pub fn new(manager: impl StoreManagerManager + 'static) -> Self {
        Self::new_with_capacity(u32::MAX, manager)
    }

    pub fn new_with_capacity(capacity: u32, manager: impl StoreManagerManager + 'static) -> Self {
        Self {
            capacity,
            manager: Box::new(manager),
        }
    }
}

impl HostComponent for KeyValueComponent {
    type Data = KeyValueDispatch;

    fn add_to_linker<T: Send>(
        linker: &mut spin_core::Linker<T>,
        get: impl Fn(&mut spin_core::Data<T>) -> &mut Self::Data + Send + Sync + Copy + 'static,
    ) -> anyhow::Result<()> {
        super::key_value::add_to_linker(linker, get)
    }

    fn build_data(&self) -> Self::Data {
        KeyValueDispatch::new_with_capacity(self.capacity)
    }
}

impl DynamicHostComponent for KeyValueComponent {
    fn update_data(&self, data: &mut Self::Data, component: &AppComponent) -> anyhow::Result<()> {
        let key_value_stores = component
            .get_metadata(crate::KEY_VALUE_STORES_KEY)?
            .unwrap_or_default();
        data.init(
            key_value_stores.into_iter().collect(),
            self.manager.get(component),
        );

        Ok(())
    }
}
