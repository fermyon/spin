use std::path::PathBuf;

use spin_app::{AppComponent, DynamicHostComponent};
use spin_core::{sqlite, HostComponent};

use crate::SqliteImpl;

#[derive(Debug, Clone)]
pub enum DatabaseLocation {
    InMemory,
    Path(PathBuf),
}

pub struct SqliteComponent {
    location: DatabaseLocation,
}

impl SqliteComponent {
    pub fn new(location: DatabaseLocation) -> Self {
        Self { location }
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
        SqliteImpl::new(self.location.clone())
    }
}

impl DynamicHostComponent for SqliteComponent {
    fn update_data(&self, _data: &mut Self::Data, _component: &AppComponent) -> anyhow::Result<()> {
        Ok(())
    }
}
