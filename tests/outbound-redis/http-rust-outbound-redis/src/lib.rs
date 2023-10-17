use anyhow::{anyhow, Context, Result};
use spin_sdk::{
    redis::{self, RedisParameter, RedisResult},
    wasi_http_component,
};
use std::collections::HashSet;

const REDIS_ADDRESS_ENV: &str = "REDIS_ADDRESS";

#[wasi_http_component]
fn test(_req: http::Request<()>) -> Result<http::Response<()>> {
    let address = std::env::var(REDIS_ADDRESS_ENV)?;
    let connection = redis::Connection::open(&address)?;

    connection
        .set("spin-example-get-set", &b"Eureka!".to_vec())
        .map_err(|_| anyhow!("Error executing Redis set command"))?;

    let payload = connection
        .get("spin-example-get-set")
        .map_err(|_| anyhow!("Error querying Redis"))?
        .context("no value found for key 'spin-example-get-set'")?;

    assert_eq!(std::str::from_utf8(&payload)?, "Eureka!");

    connection
        .set("spin-example-incr", &b"0".to_vec())
        .map_err(|_| anyhow!("Error querying Redis set command"))?;

    let int_value = connection
        .incr("spin-example-incr")
        .map_err(|_| anyhow!("Error executing Redis incr command"))?;

    assert_eq!(int_value, 1);

    let keys = vec!["spin-example-get-set".into(), "spin-example-incr".into()];

    let del_keys = connection
        .del(&keys)
        .map_err(|_| anyhow!("Error executing Redis incr command"))?;

    assert_eq!(del_keys, 2);

    connection
        .execute(
            "set",
            &[
                RedisParameter::Binary(b"spin-example".to_vec()),
                RedisParameter::Binary(b"Eureka!".to_vec()),
            ],
        )
        .map_err(|_| anyhow!("Error executing Redis set command"))?;

    connection
        .execute(
            "append",
            &[
                RedisParameter::Binary(b"spin-example".to_vec()),
                RedisParameter::Binary(b" I've got it!".to_vec()),
            ],
        )
        .map_err(|_| anyhow!("Error executing Redis append command via `execute`"))?;

    let values = connection
        .execute("get", &[RedisParameter::Binary(b"spin-example".to_vec())])
        .map_err(|_| anyhow!("Error executing Redis get command via `execute`"))?;

    assert_eq!(
        values,
        &[RedisResult::Binary(b"Eureka! I've got it!".to_vec())]
    );

    connection
        .execute(
            "set",
            &[
                RedisParameter::Binary(b"int-key".to_vec()),
                RedisParameter::Int64(0),
            ],
        )
        .map_err(|_| anyhow!("Error executing Redis set command via `execute`"))?;

    let values = connection
        .execute("incr", &[RedisParameter::Binary(b"int-key".to_vec())])
        .map_err(|_| anyhow!("Error executing Redis incr command via `execute`"))?;

    assert_eq!(values, &[RedisResult::Int64(1)]);

    let values = connection
        .execute("get", &[RedisParameter::Binary(b"int-key".to_vec())])
        .map_err(|_| anyhow!("Error executing Redis get command via `execute`"))?;

    assert_eq!(values, &[RedisResult::Binary(b"1".to_vec())]);

    connection
        .execute("del", &[RedisParameter::Binary(b"foo".to_vec())])
        .map_err(|_| anyhow!("Error executing Redis del command via `execute`"))?;

    connection
        .execute(
            "sadd",
            &[
                RedisParameter::Binary(b"foo".to_vec()),
                RedisParameter::Binary(b"bar".to_vec()),
                RedisParameter::Binary(b"baz".to_vec()),
            ],
        )
        .map_err(|_| anyhow!("Error executing Redis sadd command via `execute`"))?;

    let values = connection
        .execute("smembers", &[RedisParameter::Binary(b"foo".to_vec())])
        .map_err(|_| anyhow!("Error executing Redis smembers command via `execute`"))?;

    assert_eq!(
        values.into_iter().collect::<HashSet<_>>(),
        [
            RedisResult::Binary(b"bar".to_vec()),
            RedisResult::Binary(b"baz".to_vec())
        ]
        .into_iter()
        .collect::<HashSet<_>>()
    );

    connection
        .execute(
            "srem",
            &[
                RedisParameter::Binary(b"foo".to_vec()),
                RedisParameter::Binary(b"baz".to_vec()),
            ],
        )
        .map_err(|_| anyhow!("Error executing Redis srem command via `execute`"))?;

    let values = connection
        .execute("smembers", &[RedisParameter::Binary(b"foo".to_vec())])
        .map_err(|_| anyhow!("Error executing Redis smembers command via `execute`"))?;

    assert_eq!(
        values.into_iter().collect::<HashSet<_>>(),
        [RedisResult::Binary(b"bar".to_vec()),]
            .into_iter()
            .collect::<HashSet<_>>()
    );

    Ok(http::Response::builder().status(204).body(())?)
}
