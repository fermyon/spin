use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use once_cell::sync::OnceCell;
use rusqlite::Connection;
use spin_app::{AppComponent, DynamicHostComponent};
use spin_core::{sqlite, HostComponent};

use crate::SqliteImpl;

#[derive(Debug, Clone)]
pub enum DatabaseLocation {
    InMemory,
    Path(PathBuf),
}

/// A connection to a sqlite database
pub struct SqliteConnection {
    location: DatabaseLocation,
    connection: OnceCell<Arc<Mutex<Connection>>>,
}

impl SqliteConnection {
    pub fn new(location: DatabaseLocation) -> Self {
        Self {
            location,
            connection: OnceCell::new(),
        }
    }
}

impl ConnectionManager for SqliteConnection {
    fn get_connection(&self) -> Result<Arc<Mutex<Connection>>, sqlite::Error> {
        let connection = self
            .connection
            .get_or_try_init(|| -> Result<_, sqlite::Error> {
                let c = match &self.location {
                    DatabaseLocation::InMemory => Connection::open_in_memory(),
                    DatabaseLocation::Path(path) => Connection::open(path),
                }
                .map_err(|e| sqlite::Error::Io(e.to_string()))?;
                Ok(Arc::new(Mutex::new(c)))
            })?
            .clone();
        Ok(connection)
    }
}

pub trait ConnectionManager: Send + Sync {
    fn get_connection(&self) -> Result<Arc<Mutex<Connection>>, sqlite::Error>;
}

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
    type Data = super::SqliteImpl;

    fn add_to_linker<T: Send>(
        linker: &mut spin_core::Linker<T>,
        get: impl Fn(&mut spin_core::Data<T>) -> &mut Self::Data + Send + Sync + Copy + 'static,
    ) -> anyhow::Result<()> {
        sqlite::add_to_linker(linker, get)
    }

    fn build_data(&self) -> Self::Data {
        SqliteImpl::new(self.connection_managers.clone())
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
