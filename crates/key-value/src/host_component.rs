use crate::{KeyValueDispatch, StoreManager, KEY_VALUE_STORES_KEY};
use anyhow::anyhow;
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
        super::key_value::add_to_linker(linker, get)?;
        spin_world::v1::key_value::add_to_linker(linker, get)
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

    fn validate_app(&self, app: &spin_app::App) -> anyhow::Result<()> {
        let mut errors = vec![];

        for component in app.components() {
            let store_manager = self.manager.get(&component);
            for allowed in component
                .get_metadata(KEY_VALUE_STORES_KEY)?
                .unwrap_or_default()
            {
                if !store_manager.is_defined(&allowed) {
                    let err = format!("- Component {} uses store '{allowed}'", component.id());
                    errors.push(err);
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            let prologue = vec![
                "One or more components use key-value stores which are not defined.",
                "Check the spelling, or pass a runtime configuration file that defines these stores.",
                "See https://developer.fermyon.com/spin/dynamic-configuration#key-value-store-runtime-configuration",
                "Details:",
            ];
            let lines: Vec<_> = prologue
                .into_iter()
                .map(|s| s.to_owned())
                .chain(errors)
                .collect();
            Err(anyhow!(lines.join("\n")))
        }
    }
}
