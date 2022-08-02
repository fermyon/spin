use outbound_redis::*;
use redis::Commands;

pub use outbound_redis::add_to_linker;
use spin_engine::{
    host_component::{HostComponent, HostComponentsStateHandle},
    RuntimeContext,
};
use wit_bindgen_wasmtime::wasmtime::Linker;

wit_bindgen_wasmtime::export!("../../wit/ephemeral/outbound-redis.wit");

/// A simple implementation to support outbound Redis commands.
#[derive(Default, Clone)]
pub struct OutboundRedis;

impl HostComponent for OutboundRedis {
    type State = Self;

    fn add_to_linker<T>(
        linker: &mut Linker<RuntimeContext<T>>,
        state_handle: HostComponentsStateHandle<Self::State>,
    ) -> anyhow::Result<()> {
        add_to_linker(linker, move |ctx| state_handle.get_mut(ctx))
    }

    fn build_state(
        &self,
        _component: &spin_manifest::CoreComponent,
    ) -> anyhow::Result<Self::State> {
        Ok(Self)
    }
}

impl outbound_redis::OutboundRedis for OutboundRedis {
    fn publish(&mut self, address: &str, channel: &str, payload: &[u8]) -> Result<(), Error> {
        let client = redis::Client::open(address).map_err(|_| Error::Error)?;
        let mut pubsub_conn = client.get_connection().map_err(|_| Error::Error)?;
        pubsub_conn
            .publish(channel, payload)
            .map_err(|_| Error::Error)?;
        Ok(())
    }

    fn get(&mut self, address: &str, key: &str) -> Result<Vec<u8>, Error> {
        let client = redis::Client::open(address).map_err(|_| Error::Error)?;
        let mut conn = client.get_connection().map_err(|_| Error::Error)?;
        let value = conn.get(key).map_err(|_| Error::Error)?;
        Ok(value)
    }

    fn set(&mut self, address: &str, key: &str, value: &[u8]) -> Result<(), Error> {
        let client = redis::Client::open(address).map_err(|_| Error::Error)?;
        let mut conn = client.get_connection().map_err(|_| Error::Error)?;
        conn.set(key, value).map_err(|_| Error::Error)?;
        Ok(())
    }

    fn incr(&mut self, address: &str, key: &str) -> Result<i64, Error> {
        let client = redis::Client::open(address).map_err(|_| Error::Error)?;
        let mut conn = client.get_connection().map_err(|_| Error::Error)?;
        let value = conn.incr(key, 1).map_err(|_| Error::Error)?;
        Ok(value)
    }
}
