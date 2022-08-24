use outbound_redis::*;
use owning_ref::RwLockReadGuardRef;
use redis::Commands;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
};

pub use outbound_redis::add_to_linker;
use spin_engine::{
    host_component::{HostComponent, HostComponentsStateHandle},
    RuntimeContext,
};
use wit_bindgen_wasmtime::wasmtime::Linker;

wit_bindgen_wasmtime::export!("../../wit/ephemeral/outbound-redis.wit");

/// A simple implementation to support outbound Redis commands.
pub struct OutboundRedis {
    pub connections: Arc<RwLock<HashMap<String, Mutex<redis::Connection>>>>,
}

impl HostComponent for OutboundRedis {
    type State = Self;

    fn add_to_linker<T>(
        linker: &mut Linker<RuntimeContext<T>>,
        state_handle: HostComponentsStateHandle<Self::State>,
    ) -> anyhow::Result<()> {
        add_to_linker(linker, move |ctx| state_handle.get_mut(ctx))
    }

    fn build_state(&self, component: &spin_manifest::CoreComponent) -> anyhow::Result<Self::State> {
        let mut conn_map = HashMap::new();
        if let Some(address) = component.wasm.environment.get("REDIS_ADDRESS") {
            let client = redis::Client::open(address.to_string())?;
            let conn = client.get_connection()?;
            conn_map.insert(address.to_owned(), Mutex::new(conn));
        }
        Ok(Self {
            connections: Arc::new(RwLock::new(conn_map)),
        })
    }
}

impl outbound_redis::OutboundRedis for OutboundRedis {
    fn publish(&mut self, address: &str, channel: &str, payload: &[u8]) -> Result<(), Error> {
        let conn_map = self.get_reused_conn_map(address)?;
        let mut conn = conn_map
            .get(address)
            .unwrap()
            .lock()
            .map_err(|_| Error::Error)?;
        conn.publish(channel, payload).map_err(|_| Error::Error)?;
        Ok(())
    }

    fn get(&mut self, address: &str, key: &str) -> Result<Vec<u8>, Error> {
        let conn_map = self.get_reused_conn_map(address)?;
        let mut conn = conn_map
            .get(address)
            .unwrap()
            .lock()
            .map_err(|_| Error::Error)?;
        let value = conn.get(key).map_err(|_| Error::Error)?;
        Ok(value)
    }

    fn set(&mut self, address: &str, key: &str, value: &[u8]) -> Result<(), Error> {
        let conn_map = self.get_reused_conn_map(address)?;
        let mut conn = conn_map
            .get(address)
            .unwrap()
            .lock()
            .map_err(|_| Error::Error)?;
        conn.set(key, value).map_err(|_| Error::Error)?;
        Ok(())
    }

    fn incr(&mut self, address: &str, key: &str) -> Result<i64, Error> {
        let conn_map = self.get_reused_conn_map(address)?;
        let mut conn = conn_map
            .get(address)
            .unwrap()
            .lock()
            .map_err(|_| Error::Error)?;
        let value = conn.incr(key, 1).map_err(|_| Error::Error)?;
        Ok(value)
    }
}

impl OutboundRedis {
    fn get_reused_conn_map<'ret, 'me: 'ret, 'c>(
        &'me mut self,
        address: &'c str,
    ) -> Result<RwLockReadGuardRef<'ret, HashMap<String, Mutex<redis::Connection>>>, Error> {
        let conn_map = self.connections.read().map_err(|_| Error::Error)?;
        if conn_map.get(address).is_some() {
            tracing::debug!("Reuse connection: {:?}", address);
            return Ok(RwLockReadGuardRef::new(conn_map));
        }
        // Get rid of our read lock
        drop(conn_map);

        let mut conn_map = self.connections.write().map_err(|_| Error::Error)?;
        let client = redis::Client::open(address).map_err(|_| Error::Error)?;
        let conn = client.get_connection().map_err(|_| Error::Error)?;
        tracing::debug!("Build new connection: {:?}", address);
        conn_map.insert(address.to_string(), Mutex::new(conn));
        // Get rid of our write lock
        drop(conn_map);

        let conn_map = self.connections.read().map_err(|_| Error::Error)?;
        Ok(RwLockReadGuardRef::new(conn_map))
    }
}
