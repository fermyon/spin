use anyhow::{anyhow, Result};
use spin_sdk::{
    http::{Request, Response},
    http_component, redis,
};

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

    let del_keys = redis::del(&address, &keys)
        .map_err(|_| anyhow!("Error executing Redis incr command"))?;

    assert_eq!(del_keys, 2);

    Ok(http::Response::builder().status(204).body(None)?)
}
