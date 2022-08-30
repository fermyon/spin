use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use redis::{aio::Connection, AsyncCommands};
use spin_core::{HostComponent, Linker};
use tokio::sync::{Mutex, RwLock};
use wit_bindgen_wasmtime::async_trait;

wit_bindgen_wasmtime::export!({paths: ["../../wit/ephemeral/outbound-redis.wit"], async: *});
use outbound_redis::Error;

#[derive(Clone, Default)]
pub struct OutboundRedis {
    connections: Arc<RwLock<HashMap<String, Arc<Mutex<Connection>>>>>,
}

impl HostComponent for OutboundRedis {
    type Data = Self;

    fn add_to_linker<T: Send>(
        linker: &mut Linker<T>,
        get: impl Fn(&mut spin_core::Data<T>) -> &mut Self::Data + Send + Sync + Copy + 'static,
    ) -> anyhow::Result<()> {
        crate::outbound_redis::add_to_linker(linker, get)
    }

    fn build_data(&self) -> Self::Data {
        self.clone()
    }
}

#[async_trait]
impl outbound_redis::OutboundRedis for OutboundRedis {
    async fn publish(&mut self, address: &str, channel: &str, payload: &[u8]) -> Result<(), Error> {
        let conn = self.get_conn(address).await.map_err(log_error)?;
        conn.lock()
            .await
            .publish(channel, payload)
            .await
            .map_err(log_error)?;
        Ok(())
    }

    async fn get(&mut self, address: &str, key: &str) -> Result<Vec<u8>, Error> {
        let conn = self.get_conn(address).await.map_err(log_error)?;
        let value = conn.lock().await.get(key).await.map_err(log_error)?;
        Ok(value)
    }

    async fn set(&mut self, address: &str, key: &str, value: &[u8]) -> Result<(), Error> {
        let conn = self.get_conn(address).await.map_err(log_error)?;
        conn.lock().await.set(key, value).await.map_err(log_error)?;
        Ok(())
    }

    async fn incr(&mut self, address: &str, key: &str) -> Result<i64, Error> {
        let conn = self.get_conn(address).await.map_err(log_error)?;
        let value = conn.lock().await.incr(key, 1).await.map_err(log_error)?;
        Ok(value)
    }
}

impl OutboundRedis {
    async fn get_conn(&self, address: &str) -> Result<Arc<Mutex<Connection>>> {
        let conn_map = self.connections.read().await;
        let conn = if let Some(conn) = conn_map.get(address) {
            conn.clone()
        } else {
            let conn = redis::Client::open(address)?.get_async_connection().await?;
            let conn = Arc::new(Mutex::new(conn));
            self.connections
                .write()
                .await
                .insert(address.to_string(), conn.clone());
            conn
        };
        Ok(conn)
    }
}

fn log_error(err: impl std::fmt::Debug) -> Error {
    tracing::warn!("Outbound Redis error: {err:?}");
    Error::Error
}
