use crate::{Impl, KeyValueDispatch};
use spin_app::DynamicHostComponent;
use spin_core::HostComponent;
use std::{collections::HashMap, sync::Arc};

pub const KEY_VALUE_STORES_METADATA_KEY: &str = "key_value_stores";

pub struct KeyValueComponent {
    impls: Arc<HashMap<String, Box<dyn Impl>>>,
}

impl KeyValueComponent {
    pub fn new(impls: Arc<HashMap<String, Box<dyn Impl>>>) -> Self {
        Self { impls }
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
        KeyValueDispatch::new(self.impls.clone())
    }
}

impl DynamicHostComponent for KeyValueComponent {
    fn update_data(
        &self,
        data: &mut Self::Data,
        component: &spin_app::AppComponent,
    ) -> anyhow::Result<()> {
        data.allowed_stores = component
            .get_metadata::<Vec<String>>(KEY_VALUE_STORES_METADATA_KEY)?
            .unwrap_or_default()
            .into_iter()
            .collect();

        Ok(())
    }
}
