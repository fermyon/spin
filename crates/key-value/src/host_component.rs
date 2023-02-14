use crate::{KeyValueDispatch, StoreManager};
use spin_app::{App, AppComponent, DynamicHostComponent};
use spin_core::HostComponent;
use std::sync::Arc;

pub const KEY_VALUE_STORES_METADATA_KEY: &str = "key_value_stores";

pub trait StoreManagerManager: Sync + Send {
    fn get(&self, app: &App) -> Arc<dyn StoreManager>;
}

impl<F: (Fn(&App) -> Arc<dyn StoreManager>) + Sync + Send> StoreManagerManager for F {
    fn get(&self, app: &App) -> Arc<dyn StoreManager> {
        self(app)
    }
}

/// Help the rustc type inference engine understand that the specified closure has a higher-order bound so it can
/// be used as a [`StoreManagerManager`].
///
/// See https://stackoverflow.com/a/46198877 for details.
pub fn manager<F: for<'a> Fn(&'a App) -> Arc<dyn StoreManager>>(f: F) -> F {
    f
}

pub struct KeyValueComponent {
    manager: Box<dyn StoreManagerManager>,
}

impl KeyValueComponent {
    pub fn new(manager: impl StoreManagerManager + 'static) -> Self {
        Self {
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
        KeyValueDispatch::new()
    }
}

impl DynamicHostComponent for KeyValueComponent {
    fn update_data(&self, data: &mut Self::Data, component: &AppComponent) -> anyhow::Result<()> {
        data.init(
            component
                .get_metadata::<Vec<String>>(KEY_VALUE_STORES_METADATA_KEY)?
                .unwrap_or_default()
                .into_iter()
                .collect(),
            self.manager.get(component.app),
        );

        Ok(())
    }
}
