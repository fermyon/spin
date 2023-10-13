use anyhow::{ensure, Result};
use itertools::sorted;
use spin_sdk::{
    http_component,
    key_value::{Error, Store},
};

#[http_component]
fn handle_request(req: http::Request<()>) -> Result<http::Response<()>> {
    // TODO: once we allow users to pass non-default stores, test that opening
    // an allowed-but-non-existent one returns Error::NoSuchStore
    ensure!(matches!(Store::open("forbidden"), Err(Error::AccessDenied)));

    let query = req
        .uri()
        .query()
        .expect("Should have a testkey query string");
    let query: std::collections::HashMap<String, String> = serde_qs::from_str(query)?;
    let init_key = query
        .get("testkey")
        .expect("Should have a testkey query string");
    let init_val = query
        .get("testval")
        .expect("Should have a testval query string");

    let store = Store::open_default()?;

    store.delete("bar")?;

    ensure!(!store.exists("bar")?);

    ensure!(matches!(store.get("bar"), Err(Error::NoSuchKey)));

    store.set("bar", b"baz")?;

    ensure!(store.exists("bar")?);

    ensure!(b"baz" as &[_] == &store.get("bar")?);

    store.set("bar", b"wow")?;

    ensure!(b"wow" as &[_] == &store.get("bar")?);

    ensure!(
        init_val.as_bytes() == store.get(init_key)?,
        "Expected to look up {init_key} and get {init_val} but actually got {}",
        String::from_utf8_lossy(&store.get(init_key)?)
    );

    ensure!(
        sorted(vec!["bar".to_owned(), init_key.to_owned()]).collect::<Vec<_>>()
            == sorted(store.get_keys()?).collect::<Vec<_>>(),
        "Expected exectly keys 'bar' and '{}' but got '{:?}'",
        init_key,
        &store.get_keys()?
    );

    store.delete("bar")?;
    store.delete(init_key)?;

    ensure!(&[] as &[String] == &store.get_keys()?);

    ensure!(!store.exists("bar")?);

    ensure!(matches!(store.get("bar"), Err(Error::NoSuchKey)));

    Ok(http::Response::builder().status(200).body(())?)
}
