mod host_component;

use std::collections::{hash_map::Entry, HashMap};

use anyhow::Result;
use redis::{aio::Connection, AsyncCommands, FromRedisValue, Value};
use spin_core::{
    async_trait, redis as outbound_redis,
    redis_types::{Error, RedisParameter, RedisResult},
};

pub use host_component::OutboundRedisComponent;

struct RedisResults(Vec<RedisResult>);

impl FromRedisValue for RedisResults {
    fn from_redis_value(value: &Value) -> redis::RedisResult<Self> {
        fn append(values: &mut Vec<RedisResult>, value: &Value) {
            match value {
                Value::Nil | Value::Okay => (),
                Value::Int(v) => values.push(RedisResult::Int64(*v)),
                Value::Data(bytes) => values.push(RedisResult::Binary(bytes.to_owned())),
                Value::Bulk(bulk) => bulk.iter().for_each(|value| append(values, value)),
                Value::Status(message) => values.push(RedisResult::Status(message.to_owned())),
            }
        }

        let mut values = Vec::new();
        append(&mut values, value);
        Ok(RedisResults(values))
    }
}

#[derive(Default)]
pub struct OutboundRedis {
    connections: HashMap<String, Connection>,
}

#[async_trait]
impl outbound_redis::Host for OutboundRedis {
    async fn publish(
        &mut self,
        address: String,
        channel: String,
        payload: Vec<u8>,
    ) -> Result<Result<(), Error>> {
        Ok(async {
            let conn = self.get_conn(&address).await.map_err(log_error)?;
            conn.publish(&channel, &payload).await.map_err(log_error)?;
            Ok(())
        }
        .await)
    }

    async fn get(&mut self, address: String, key: String) -> Result<Result<Vec<u8>, Error>> {
        Ok(async {
            let conn = self.get_conn(&address).await.map_err(log_error)?;
            let value = conn.get(&key).await.map_err(log_error)?;
            Ok(value)
        }
        .await)
    }

    async fn set(
        &mut self,
        address: String,
        key: String,
        value: Vec<u8>,
    ) -> Result<Result<(), Error>> {
        Ok(async {
            let conn = self.get_conn(&address).await.map_err(log_error)?;
            conn.set(&key, &value).await.map_err(log_error)?;
            Ok(())
        }
        .await)
    }

    async fn incr(&mut self, address: String, key: String) -> Result<Result<i64, Error>> {
        Ok(async {
            let conn = self.get_conn(&address).await.map_err(log_error)?;
            let value = conn.incr(&key, 1).await.map_err(log_error)?;
            Ok(value)
        }
        .await)
    }

    async fn del(&mut self, address: String, keys: Vec<String>) -> Result<Result<i64, Error>> {
        Ok(async {
            let conn = self.get_conn(&address).await.map_err(log_error)?;
            let value = conn.del(&keys).await.map_err(log_error)?;
            Ok(value)
        }
        .await)
    }

    async fn sadd(
        &mut self,
        address: String,
        key: String,
        values: Vec<String>,
    ) -> Result<Result<i64, Error>> {
        Ok(async {
            let conn = self.get_conn(&address).await.map_err(log_error)?;
            let value = conn.sadd(&key, &values).await.map_err(log_error)?;
            Ok(value)
        }
        .await)
    }

    async fn smembers(
        &mut self,
        address: String,
        key: String,
    ) -> Result<Result<Vec<String>, Error>> {
        Ok(async {
            let conn = self.get_conn(&address).await.map_err(log_error)?;
            let value = conn.smembers(&key).await.map_err(log_error)?;
            Ok(value)
        }
        .await)
    }

    async fn srem(
        &mut self,
        address: String,
        key: String,
        values: Vec<String>,
    ) -> Result<Result<i64, Error>> {
        Ok(async {
            let conn = self.get_conn(&address).await.map_err(log_error)?;
            let value = conn.srem(&key, &values).await.map_err(log_error)?;
            Ok(value)
        }
        .await)
    }

    async fn execute(
        &mut self,
        address: String,
        command: String,
        arguments: Vec<RedisParameter>,
    ) -> Result<Result<Vec<RedisResult>, Error>> {
        Ok(async {
            let conn = self.get_conn(&address).await.map_err(log_error)?;
            let mut cmd = redis::cmd(&command);
            arguments.iter().for_each(|value| match value {
                RedisParameter::Int64(v) => {
                    cmd.arg(v);
                }
                RedisParameter::Binary(v) => {
                    cmd.arg(v);
                }
            });

            cmd.query_async::<_, RedisResults>(conn)
                .await
                .map(|values| values.0)
                .map_err(log_error)
        }
        .await)
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
