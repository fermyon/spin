mod host_component;

use std::collections::HashMap;

pub use host_component::SqliteComponent;
use rand::Rng;
use spin_core::{
    async_trait,
    sqlite::{self, Host},
};

pub struct SqliteImpl {
    connections: HashMap<sqlite::Connection, rusqlite::Connection>,
}

impl SqliteImpl {
    pub fn new() -> Self {
        Self {
            connections: HashMap::default(),
        }
    }

    pub fn component_init(&mut self) {}
}

#[async_trait]
impl Host for SqliteImpl {
    async fn open(
        &mut self,
        name: String,
    ) -> anyhow::Result<Result<spin_core::sqlite::Connection, spin_core::sqlite::Error>> {
        println!("Opening..");
        let conn = rusqlite::Connection::open_in_memory()?;
        let mut rng = rand::thread_rng();
        let c: sqlite::Connection = rng.gen();
        self.connections.insert(c, conn);
        Ok(Ok(c))
    }

    async fn execute(
        &mut self,
        connection: sqlite::Connection,
        query: String,
    ) -> anyhow::Result<Result<(), sqlite::Error>> {
        if let Some(c) = self.connections.get(&connection) {
            c.execute(&query, []);
            let s: Result<u32, _> =
                c.query_row("SELECT COUNT(name) FROM person;", [], |row| row.get(0));
            println!("{s:?}");
        }
        Ok(Ok(()))
    }

    async fn close(&mut self, connection: spin_core::sqlite::Connection) -> anyhow::Result<()> {
        let _ = self.connections.remove(&connection);
        Ok(())
    }
}
