use anyhow::{anyhow, Result};
use spin_sdk::{
    http_component,
    redis::{self, RedisParameter, RedisResult},
};
use std::collections::HashSet;

const REDIS_ADDRESS_ENV: &str = "REDIS_ADDRESS";

#[http_component]
fn test(_req: http::Request<()>) -> Result<http::Response<()>> {
    let address = std::env::var(REDIS_ADDRESS_ENV)?;

    redis::set(&address, "spin-example-get-set", &b"Eureka!".to_vec())
        .map_err(|_| anyhow!("Error executing Redis set command"))?;

    let payload = redis::get(&address, "spin-example-get-set")
        .map_err(|_| anyhow!("Error querying Redis"))?;

    assert_eq!(std::str::from_utf8(&payload).unwrap(), "Eureka!");

    redis::set(&address, "spin-example-incr", &b"0".to_vec())
        .map_err(|_| anyhow!("Error querying Redis set command"))?;

    let int_value = redis::incr(&address, "spin-example-incr")
        .map_err(|_| anyhow!("Error executing Redis incr command"))?;

    assert_eq!(int_value, 1);

    let keys = vec!["spin-example-get-set".into(), "spin-example-incr".into()];

    let del_keys =
        redis::del(&address, &keys).map_err(|_| anyhow!("Error executing Redis incr command"))?;

    assert_eq!(del_keys, 2);

    redis::execute(
        &address,
        "set",
        &[
            RedisParameter::Binary(b"spin-example".to_vec()),
            RedisParameter::Binary(b"Eureka!".to_vec()),
        ],
    )
    .map_err(|_| anyhow!("Error executing Redis set command"))?;

    redis::execute(
        &address,
        "append",
        &[
            RedisParameter::Binary(b"spin-example".to_vec()),
            RedisParameter::Binary(b" I've got it!".to_vec()),
        ],
    )
    .map_err(|_| anyhow!("Error executing Redis append command via `execute`"))?;

    let values = redis::execute(
        &address,
        "get",
        &[RedisParameter::Binary(b"spin-example".to_vec())],
    )
    .map_err(|_| anyhow!("Error executing Redis get command via `execute`"))?;

    assert_eq!(
        values,
        &[RedisResult::Binary(b"Eureka! I've got it!".to_vec())]
    );

    redis::execute(
        &address,
        "set",
        &[
            RedisParameter::Binary(b"int-key".to_vec()),
            RedisParameter::Int64(0),
        ],
    )
    .map_err(|_| anyhow!("Error executing Redis set command via `execute`"))?;

    let values = redis::execute(
        &address,
        "incr",
        &[RedisParameter::Binary(b"int-key".to_vec())],
    )
    .map_err(|_| anyhow!("Error executing Redis incr command via `execute`"))?;

    assert_eq!(values, &[RedisResult::Int64(1)]);

    let values = redis::execute(
        &address,
        "get",
        &[RedisParameter::Binary(b"int-key".to_vec())],
    )
    .map_err(|_| anyhow!("Error executing Redis get command via `execute`"))?;

    assert_eq!(values, &[RedisResult::Binary(b"1".to_vec())]);

    redis::execute(&address, "del", &[RedisParameter::Binary(b"foo".to_vec())])
        .map_err(|_| anyhow!("Error executing Redis del command via `execute`"))?;

    redis::execute(
        &address,
        "sadd",
        &[
            RedisParameter::Binary(b"foo".to_vec()),
            RedisParameter::Binary(b"bar".to_vec()),
            RedisParameter::Binary(b"baz".to_vec()),
        ],
    )
    .map_err(|_| anyhow!("Error executing Redis sadd command via `execute`"))?;

    let values = redis::execute(
        &address,
        "smembers",
        &[RedisParameter::Binary(b"foo".to_vec())],
    )
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

    redis::execute(
        &address,
        "srem",
        &[
            RedisParameter::Binary(b"foo".to_vec()),
            RedisParameter::Binary(b"baz".to_vec()),
        ],
    )
    .map_err(|_| anyhow!("Error executing Redis srem command via `execute`"))?;

    let values = redis::execute(
        &address,
        "smembers",
        &[RedisParameter::Binary(b"foo".to_vec())],
    )
    .map_err(|_| anyhow!("Error executing Redis smembers command via `execute`"))?;

    assert_eq!(
        values.into_iter().collect::<HashSet<_>>(),
        [RedisResult::Binary(b"bar".to_vec()),]
            .into_iter()
            .collect::<HashSet<_>>()
    );

    Ok(http::Response::builder().status(204).body(())?)
}
