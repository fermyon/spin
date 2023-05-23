use spin_core::async_trait;
use spin_key_value::table;
use spin_world::sqlite::{self, Host};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use crate::ConnectionManager;

use super::Connection;

/// An implementation of the SQLite host
pub struct SqliteImpl {
    allowed_databases: HashSet<String>,
    connections: table::Table<Arc<dyn Connection>>,
    client_manager: HashMap<String, Arc<dyn ConnectionManager>>,
}

impl SqliteImpl {
    pub fn new(client_manager: HashMap<String, Arc<dyn ConnectionManager>>) -> Self {
        Self {
            connections: table::Table::new(256),
            allowed_databases: HashSet::new(),
            client_manager,
        }
    }

    pub fn component_init(&mut self, allowed_databases: HashSet<String>) {
        self.allowed_databases = allowed_databases
    }

    fn get_connection(
        &self,
        connection: sqlite::Connection,
    ) -> Result<&Arc<dyn Connection>, sqlite::Error> {
        Ok(self
            .connections
            .get(connection)
            .ok_or(sqlite::Error::InvalidConnection)?)
    }
}

#[async_trait]
impl Host for SqliteImpl {
    async fn open(
        &mut self,
        database: String,
    ) -> anyhow::Result<Result<sqlite::Connection, sqlite::Error>> {
        Ok(tokio::task::block_in_place(|| {
            if !self.allowed_databases.contains(&database) {
                return Err(sqlite::Error::AccessDenied);
            }
            self.connections
                .push(
                    self.client_manager
                        .get(&database)
                        .ok_or(sqlite::Error::NoSuchDatabase)?
                        .get_connection()?,
                )
                .map_err(|()| sqlite::Error::DatabaseFull)
        }))
    }

    async fn execute(
        &mut self,
        connection: sqlite::Connection,
        query: String,
        parameters: Vec<sqlite::Value>,
    ) -> anyhow::Result<Result<sqlite::QueryResult, sqlite::Error>> {
        Ok(tokio::task::block_in_place(|| {
            let conn = self.get_connection(connection)?;
            Ok(conn.query(&query, parameters)?)
        }))
    }

    async fn close(&mut self, connection: sqlite::Connection) -> anyhow::Result<()> {
        let _ = self.connections.remove(connection);
        Ok(())
    }
}
