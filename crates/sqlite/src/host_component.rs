use std::{collections::HashMap, sync::Arc};

use crate::{ConnectionManager, SqliteDispatch, DATABASES_KEY};
use anyhow::anyhow;
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

    fn validate_app(&self, app: &spin_app::App) -> anyhow::Result<()> {
        let mut errors = vec![];

        for component in app.components() {
            let connection_managers = &self.connection_managers;
            for allowed in component.get_metadata(DATABASES_KEY)?.unwrap_or_default() {
                if !connection_managers.contains_key(&allowed) {
                    let err = format!("- Component {} uses database '{allowed}'", component.id());
                    errors.push(err);
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            let prologue = vec![
                "One or more components use SQLite databases which are not defined.",
                "Check the spelling, or pass a runtime configuration file that defines these stores.",
                "See https://developer.fermyon.com/spin/dynamic-configuration#sqlite-storage-runtime-configuration",
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
