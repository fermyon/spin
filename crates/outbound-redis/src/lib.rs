mod host_component;

use std::collections::{hash_map::Entry, HashMap};

use anyhow::Result;
use redis::{aio::Connection, AsyncCommands, FromRedisValue, RedisResult, Value};
use wit_bindgen_wasmtime::async_trait;

pub use host_component::OutboundRedisComponent;

wit_bindgen_wasmtime::export!({paths: ["../../wit/ephemeral/outbound-redis.wit"], async: *});
use outbound_redis::{Error, ValueParam, ValueResult};

struct Values(Vec<ValueResult>);

impl FromRedisValue for Values {
    fn from_redis_value(value: &Value) -> RedisResult<Self> {
        fn append(values: &mut Vec<ValueResult>, value: &Value) {
            match value {
                Value::Nil | Value::Okay => (),
                Value::Int(v) => values.push(ValueResult::Int(*v)),
                Value::Data(bytes) => values.push(ValueResult::Data(bytes.to_owned())),
                Value::Bulk(bulk) => bulk.iter().for_each(|value| append(values, value)),
                Value::Status(message) => values.push(ValueResult::String(message.to_owned())),
            }
        }

        let mut values = Vec::new();
        append(&mut values, value);
        Ok(Values(values))
    }
}

#[derive(Default)]
pub struct OutboundRedis {
    connections: HashMap<String, Connection>,
}

#[async_trait]
impl outbound_redis::OutboundRedis for OutboundRedis {
    async fn publish(&mut self, address: &str, channel: &str, payload: &[u8]) -> Result<(), Error> {
        let conn = self.get_conn(address).await.map_err(log_error)?;
        conn.publish(channel, payload).await.map_err(log_error)?;
        Ok(())
    }

    async fn get(&mut self, address: &str, key: &str) -> Result<Vec<u8>, Error> {
        let conn = self.get_conn(address).await.map_err(log_error)?;
        let value = conn.get(key).await.map_err(log_error)?;
        Ok(value)
    }

    async fn set(&mut self, address: &str, key: &str, value: &[u8]) -> Result<(), Error> {
        let conn = self.get_conn(address).await.map_err(log_error)?;
        conn.set(key, value).await.map_err(log_error)?;
        Ok(())
    }

    async fn incr(&mut self, address: &str, key: &str) -> Result<i64, Error> {
        let conn = self.get_conn(address).await.map_err(log_error)?;
        let value = conn.incr(key, 1).await.map_err(log_error)?;
        Ok(value)
    }

    async fn del(&mut self, address: &str, keys: Vec<&str>) -> Result<i64, Error> {
        let conn = self.get_conn(address).await.map_err(log_error)?;
        let value = conn.del(keys).await.map_err(log_error)?;
        Ok(value)
    }

    async fn sadd(&mut self, address: &str, key: &str, values: Vec<&str>) -> Result<i64, Error> {
        let conn = self.get_conn(address).await.map_err(log_error)?;
        let value = conn.sadd(key, values).await.map_err(log_error)?;
        Ok(value)
    }

    async fn smembers(&mut self, address: &str, key: &str) -> Result<Vec<String>, Error> {
        let conn = self.get_conn(address).await.map_err(log_error)?;
        let value = conn.smembers(key).await.map_err(log_error)?;
        Ok(value)
    }

    async fn srem(&mut self, address: &str, key: &str, values: Vec<&str>) -> Result<i64, Error> {
        let conn = self.get_conn(address).await.map_err(log_error)?;
        let value = conn.srem(key, values).await.map_err(log_error)?;
        Ok(value)
    }

    async fn execute(
        &mut self,
        address: &str,
        command: &str,
        arguments: Vec<ValueParam<'_>>,
    ) -> Result<Vec<ValueResult>, Error> {
        let conn = self.get_conn(address).await.map_err(log_error)?;
        let mut cmd = redis::cmd(command);
        arguments.iter().for_each(|value| match value {
            ValueParam::Nil => (),
            ValueParam::String(s) => {
                cmd.arg(s);
            }
            ValueParam::Int(v) => {
                cmd.arg(v);
            }
            ValueParam::Data(v) => {
                cmd.arg(v);
            }
        });

        cmd.query_async::<_, Values>(conn)
            .await
            .map(|values| values.0)
            .map_err(log_error)
    }
}

impl OutboundRedis {
    async fn get_conn(&mut self, address: &str) -> Result<&mut Connection> {
        let conn = match self.connections.entry(address.to_string()) {
            Entry::Occupied(o) => o.into_mut(),
            Entry::Vacant(v) => {
                let conn = redis::Client::open(address)?.get_async_connection().await?;
                v.insert(conn)
            }
        };
        Ok(conn)
    }
}

fn log_error(err: impl std::fmt::Debug) -> Error {
    tracing::warn!("Outbound Redis error: {err:?}");
    Error::Error
}
