mod host_component;

use anyhow::Result;
use redis::{aio::Connection, AsyncCommands, FromRedisValue, Value};
use spin_core::{async_trait, wasmtime::component::Resource};
use spin_world::v1::redis as v1;
use spin_world::v2::redis as v2;
use v1::{Error, RedisParameter, RedisResult};
use v2::Connection as RedisConnection;

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

pub struct OutboundRedis {
    connections: table::Table<Connection>,
}

impl Default for OutboundRedis {
    fn default() -> Self {
        Self {
            connections: table::Table::new(1024)
        }
    }
}

impl v2::Host for OutboundRedis {}

#[async_trait]
impl v2::HostConnection for OutboundRedis {
    async fn open(&mut self, address: String) -> Result<Result<Resource<RedisConnection>, Error>> {
        let conn = redis::Client::open(address.as_str())?
            .get_async_connection()
            .await?;
        Ok(self.connections.push(conn).map(Resource::new_own).map_err(|_| Error::Error))
    }

    async fn publish(
        &mut self,
        connection: Resource<RedisConnection>,
        channel: String,
        payload: Vec<u8>,
    ) -> Result<Result<(), Error>> {
        Ok(async {
            let conn = self.get_conn(connection).await.map_err(log_error)?;
            conn.publish(&channel, &payload).await.map_err(log_error)?;
            Ok(())
        }
        .await)
    }

    async fn get(
        &mut self,
        connection: Resource<RedisConnection>,
        key: String,
    ) -> Result<Result<Vec<u8>, Error>> {
        Ok(async {
            let conn = self.get_conn(connection).await.map_err(log_error)?;
            let value = conn.get(&key).await.map_err(log_error)?;
            Ok(value)
        }
        .await)
    }

    async fn set(
        &mut self,
        connection: Resource<RedisConnection>,
        key: String,
        value: Vec<u8>,
    ) -> Result<Result<(), Error>> {
        Ok(async {
            let conn = self.get_conn(connection).await.map_err(log_error)?;
            conn.set(&key, &value).await.map_err(log_error)?;
            Ok(())
        }
        .await)
    }

    async fn incr(
        &mut self,
        connection: Resource<RedisConnection>,
        key: String,
    ) -> Result<Result<i64, Error>> {
        Ok(async {
            let conn = self.get_conn(connection).await.map_err(log_error)?;
            let value = conn.incr(&key, 1).await.map_err(log_error)?;
            Ok(value)
        }
        .await)
    }

    async fn del(
        &mut self,
        connection: Resource<RedisConnection>,
        keys: Vec<String>,
    ) -> Result<Result<i64, Error>> {
        Ok(async {
            let conn = self.get_conn(connection).await.map_err(log_error)?;
            let value = conn.del(&keys).await.map_err(log_error)?;
            Ok(value)
        }
        .await)
    }

    async fn sadd(
        &mut self,
        connection: Resource<RedisConnection>,
        key: String,
        values: Vec<String>,
    ) -> Result<Result<i64, Error>> {
        Ok(async {
            let conn = self.get_conn(connection).await.map_err(log_error)?;
            let value = conn.sadd(&key, &values).await.map_err(log_error)?;
            Ok(value)
        }
        .await)
    }

    async fn smembers(
        &mut self,
        connection: Resource<RedisConnection>,
        key: String,
    ) -> Result<Result<Vec<String>, Error>> {
        Ok(async {
            let conn = self.get_conn(connection).await.map_err(log_error)?;
            let value = conn.smembers(&key).await.map_err(log_error)?;
            Ok(value)
        }
        .await)
    }

    async fn srem(
        &mut self,
        connection: Resource<RedisConnection>,
        key: String,
        values: Vec<String>,
    ) -> Result<Result<i64, Error>> {
        Ok(async {
            let conn = self.get_conn(connection).await.map_err(log_error)?;
            let value = conn.srem(&key, &values).await.map_err(log_error)?;
            Ok(value)
        }
        .await)
    }

    async fn execute(
        &mut self,
        connection: Resource<RedisConnection>,
        command: String,
        arguments: Vec<RedisParameter>,
    ) -> Result<Result<Vec<RedisResult>, Error>> {
        Ok(async {
            let conn = self.get_conn(connection).await.map_err(log_error)?;
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

    fn drop(&mut self, connection: Resource<RedisConnection>) -> anyhow::Result<()> {
        self.connections.remove(connection.rep());
        Ok(())
    }
}

macro_rules! unwrap {
    ($expr:expr) => {
        match $expr {
            Ok(s) => s,
            Err(e) => return Ok(Err(e)),
        }
    };
}

#[async_trait]
impl v1::Host for OutboundRedis {
    async fn publish(
        &mut self,
        address: String,
        channel: String,
        payload: Vec<u8>,
    ) -> Result<Result<(), Error>> {
        let connection = unwrap!(<Self as v2::HostConnection>::open(self, address).await?);
        <Self as v2::HostConnection>::publish(self, connection, channel, payload).await
    }

    async fn get(&mut self, address: String, key: String) -> Result<Result<Vec<u8>, Error>> {
        let connection = unwrap!(<Self as v2::HostConnection>::open(self, address).await?);
        <Self as v2::HostConnection>::get(self, connection, key).await
    }

    async fn set(
        &mut self,
        address: String,
        key: String,
        value: Vec<u8>,
    ) -> Result<Result<(), Error>> {
        let connection = unwrap!(<Self as v2::HostConnection>::open(self, address).await?);
        <Self as v2::HostConnection>::set(self, connection, key, value).await
    }

    async fn incr(&mut self, address: String, key: String) -> Result<Result<i64, Error>> {
        let connection = unwrap!(<Self as v2::HostConnection>::open(self, address).await?);
        <Self as v2::HostConnection>::incr(self, connection, key).await
    }

    async fn del(&mut self, address: String, keys: Vec<String>) -> Result<Result<i64, Error>> {
        let connection = unwrap!(<Self as v2::HostConnection>::open(self, address).await?);
        <Self as v2::HostConnection>::del(self, connection, keys).await
    }

    async fn sadd(
        &mut self,
        address: String,
        key: String,
        values: Vec<String>,
    ) -> Result<Result<i64, Error>> {
        let connection = unwrap!(<Self as v2::HostConnection>::open(self, address).await?);
        <Self as v2::HostConnection>::sadd(self, connection, key, values).await
    }

    async fn smembers(
        &mut self,
        address: String,
        key: String,
    ) -> Result<Result<Vec<String>, Error>> {
        let connection = unwrap!(<Self as v2::HostConnection>::open(self, address).await?);
        <Self as v2::HostConnection>::smembers(self, connection, key).await
    }

    async fn srem(
        &mut self,
        address: String,
        key: String,
        values: Vec<String>,
    ) -> Result<Result<i64, Error>> {
        let connection = unwrap!(<Self as v2::HostConnection>::open(self, address).await?);
        <Self as v2::HostConnection>::srem(self, connection, key, values).await
    }

    async fn execute(
        &mut self,
        address: String,
        command: String,
        arguments: Vec<RedisParameter>,
    ) -> Result<Result<Vec<RedisResult>, Error>> {
        let connection = unwrap!(<Self as v2::HostConnection>::open(self, address).await?);
        <Self as v2::HostConnection>::execute(self, connection, command, arguments).await
    }
}

impl OutboundRedis {
    async fn get_conn(&mut self, connection: Resource<RedisConnection>) -> Result<&mut Connection> {
        Ok(self.connections.get_mut(connection.rep()).expect("could not find connection for resource"))
    }
}

fn log_error(err: impl std::fmt::Debug) -> Error {
    tracing::warn!("Outbound Redis error: {err:?}");
    Error::Error
}
