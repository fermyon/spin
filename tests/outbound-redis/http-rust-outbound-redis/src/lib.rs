use anyhow::{anyhow, Result};
use spin_sdk::{
    http::{Request, Response},
    http_component,
    redis::{self, ValueParam, ValueResult},
};
use std::collections::HashSet;

const REDIS_ADDRESS_ENV: &str = "REDIS_ADDRESS";

#[http_component]
fn test(_req: Request) -> Result<Response> {
    let address = std::env::var(REDIS_ADDRESS_ENV)?;

    redis::set(&address, "spin-example-get-set", b"Eureka!")
        .map_err(|_| anyhow!("Error executing Redis set command"))?;

    let payload = redis::get(&address, "spin-example-get-set")
        .map_err(|_| anyhow!("Error querying Redis"))?;

    assert_eq!(std::str::from_utf8(&payload).unwrap(), "Eureka!");

    redis::set(&address, "spin-example-incr", b"0")
        .map_err(|_| anyhow!("Error querying Redis set command"))?;

    let int_value = redis::incr(&address, "spin-example-incr")
        .map_err(|_| anyhow!("Error executing Redis incr command"))?;

    assert_eq!(int_value, 1);

    let keys = vec!["spin-example-get-set", "spin-example-incr"];

    let del_keys =
        redis::del(&address, &keys).map_err(|_| anyhow!("Error executing Redis incr command"))?;

    assert_eq!(del_keys, 2);

    redis::execute(
        &address,
        "set",
        &[
            ValueParam::String("spin-example"),
            ValueParam::Data(b"Eureka!"),
        ],
    )
    .map_err(|_| anyhow!("Error executing Redis set command"))?;

    redis::execute(
        &address,
        "append",
        &[
            ValueParam::String("spin-example"),
            ValueParam::Data(b" I've got it!"),
        ],
    )
    .map_err(|_| anyhow!("Error executing Redis set command"))?;

    let values = redis::execute(&address, "get", &[ValueParam::String("spin-example")])
        .map_err(|_| anyhow!("Error executing Redis get command"))?;

    assert_eq!(
        values,
        &[ValueResult::Data(b"Eureka! I've got it!".to_vec())]
    );

    redis::execute(
        &address,
        "set",
        &[ValueParam::String("int-key"), ValueParam::Int(0)],
    )
    .map_err(|_| anyhow!("Error executing Redis set command"))?;

    let values = redis::execute(&address, "incr", &[ValueParam::String("int-key")])
        .map_err(|_| anyhow!("Error executing Redis incr command"))?;

    assert_eq!(values, &[ValueResult::Int(1)]);

    let values = redis::execute(&address, "get", &[ValueParam::String("int-key")])
        .map_err(|_| anyhow!("Error executing Redis get command"))?;

    assert_eq!(values, &[ValueResult::Data(b"1".to_vec())]);

    redis::execute(&address, "del", &[ValueParam::String("foo")])
        .map_err(|_| anyhow!("Error executing Redis del command"))?;

    redis::execute(
        &address,
        "sadd",
        &[
            ValueParam::String("foo"),
            ValueParam::String("bar"),
            ValueParam::String("baz"),
        ],
    )
    .map_err(|_| anyhow!("Error executing Redis sadd command"))?;

    let values = redis::execute(&address, "smembers", &[ValueParam::String("foo")])
        .map_err(|_| anyhow!("Error executing Redis smembers command"))?;

    assert_eq!(
        values.into_iter().collect::<HashSet<_>>(),
        [
            ValueResult::Data(b"bar".to_vec()),
            ValueResult::Data(b"baz".to_vec())
        ]
        .into_iter()
        .collect::<HashSet<_>>()
    );

    redis::execute(
        &address,
        "srem",
        &[ValueParam::String("foo"), ValueParam::String("baz")],
    )
    .map_err(|_| anyhow!("Error executing Redis srem command"))?;

    let values = redis::execute(&address, "smembers", &[ValueParam::String("foo")])
        .map_err(|_| anyhow!("Error executing Redis smembers command"))?;

    assert_eq!(
        values.into_iter().collect::<HashSet<_>>(),
        [ValueResult::Data(b"bar".to_vec()),]
            .into_iter()
            .collect::<HashSet<_>>()
    );

    Ok(http::Response::builder().status(204).body(None)?)
}
