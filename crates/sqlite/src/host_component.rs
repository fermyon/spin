use std::{collections::HashMap, sync::Arc};

use crate::{ConnectionManager, SqliteDispatch};
use spin_app::{AppComponent, DynamicHostComponent};
use spin_core::HostComponent;
use spin_world::sqlite;

pub struct SqliteComponent {
    connection_managers: HashMap<String, Arc<dyn ConnectionManager>>,
}

impl SqliteComponent {
    pub fn new(connection_managers: HashMap<String, Arc<dyn ConnectionManager>>) -> Self {
        Self {
            connection_managers,
        }
    }
}

impl HostComponent for SqliteComponent {
    type Data = super::SqliteDispatch;

    fn add_to_linker<T: Send>(
        linker: &mut spin_core::Linker<T>,
        get: impl Fn(&mut spin_core::Data<T>) -> &mut Self::Data + Send + Sync + Copy + 'static,
    ) -> anyhow::Result<()> {
        sqlite::add_to_linker(linker, get)
    }

    fn build_data(&self) -> Self::Data {
        SqliteDispatch::new(self.connection_managers.clone())
    }
}

impl DynamicHostComponent for SqliteComponent {
    fn update_data(&self, data: &mut Self::Data, component: &AppComponent) -> anyhow::Result<()> {
        let allowed_databases = component
            .get_metadata(crate::DATABASES_KEY)?
            .unwrap_or_default();
        data.component_init(allowed_databases);
        // TODO: allow dynamically updating connection manager
        Ok(())
    }
}
