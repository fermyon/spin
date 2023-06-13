use std::sync::Arc;

use crate::{ConnectionsStore, SqliteDispatch, DATABASES_KEY};
use anyhow::anyhow;
use spin_app::{AppComponent, DynamicHostComponent};
use spin_core::HostComponent;
use spin_world::sqlite;

type InitConnectionsStore = dyn (Fn(&AppComponent) -> Arc<dyn ConnectionsStore>) + Sync + Send;

pub struct SqliteComponent {
    /// Function that can be called when a `ConnectionsStore` is needed
    init_connections_store: Box<InitConnectionsStore>,
}

impl SqliteComponent {
    pub fn new<F>(init_connections_store: F) -> Self
    where
        F: (Fn(&AppComponent) -> Arc<dyn ConnectionsStore>) + Sync + Send + 'static,
    {
        Self {
            init_connections_store: Box::new(init_connections_store),
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
        // To initialize `SqliteDispatch` we need a `ConnectionsStore`, but we can't build one
        // until we have a `ComponentApp`. That's fine though as we'll have one `DynamicHostComponent::update_data`.
        // The Noop implementation will never get called.
        struct Noop;
        impl ConnectionsStore for Noop {
            fn get_connection(
                &self,
                _database: &str,
            ) -> Result<Option<Arc<(dyn crate::Connection + 'static)>>, spin_world::sqlite::Error>
            {
                debug_assert!(false, "`Noop` `ConnectionsStore` was called");
                Ok(None)
            }

            fn has_connection_for(&self, _database: &str) -> bool {
                debug_assert!(false, "`Noop` `ConnectionsStore` was called");
                false
            }
        }
        SqliteDispatch::new(Arc::new(Noop))
    }
}

impl DynamicHostComponent for SqliteComponent {
    fn update_data(&self, data: &mut Self::Data, component: &AppComponent) -> anyhow::Result<()> {
        let allowed_databases = component
            .get_metadata(crate::DATABASES_KEY)?
            .unwrap_or_default();
        data.component_init(allowed_databases, (self.init_connections_store)(component));
        Ok(())
    }

    fn validate_app(&self, app: &spin_app::App) -> anyhow::Result<()> {
        let mut errors = vec![];

        for component in app.components() {
            let connections_store = (self.init_connections_store)(&component);
            for allowed in component.get_metadata(DATABASES_KEY)?.unwrap_or_default() {
                if !connections_store.has_connection_for(&allowed) {
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
