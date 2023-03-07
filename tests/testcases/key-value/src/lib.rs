use anyhow::{ensure, Result};
use spin_sdk::{
    http::{Request, Response},
    http_component,
    key_value::{Error, Store},
};

#[http_component]
fn handle_request(_req: Request) -> Result<Response> {
    // TODO: once we allow users to pass non-default stores, test that opening
    // an allowed-but-non-existent one returns Error::NoSuchStore
    ensure!(matches!(Store::open("forbidden"), Err(Error::AccessDenied)));

    let store = Store::open_default()?;

    store.delete("bar")?;

    ensure!(!store.exists("bar")?);

    ensure!(matches!(store.get("bar"), Err(Error::NoSuchKey)));

    store.set("bar", b"baz")?;

    ensure!(store.exists("bar")?);

    ensure!(b"baz" as &[_] == &store.get("bar")?);

    store.set("bar", b"wow")?;

    ensure!(b"wow" as &[_] == &store.get("bar")?);

    ensure!(&["bar".to_owned()] as &[_] == &store.get_keys()?);

    store.delete("bar")?;

    ensure!(&[] as &[String] == &store.get_keys()?);

    ensure!(!store.exists("bar")?);

    ensure!(matches!(store.get("bar"), Err(Error::NoSuchKey)));

    Ok(http::Response::builder().status(200).body(None)?)
}
