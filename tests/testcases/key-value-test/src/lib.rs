use anyhow::Result;
use http::{Method, StatusCode};
use spin_sdk::{
    http::{Request, Response},
    http_component,
    key_value::{Error, Store},
};

#[http_component]
fn key_value_test(req: Request) -> Result<Response> {
    let store = Store::open_default()?;
    let (status, body) = match *req.method() {
        Method::POST => {
            println!("Adding key and value to store");
            store.set(req.uri().path(), req.body().as_deref().unwrap_or(&[]))?;
            (StatusCode::OK, Some("Value set".into()))
        }
        Method::GET => {
            match req.uri().path() {
                // Run a within request key/value test
                "/test" => {
                    full_test(&store)?;
                    (StatusCode::OK, Some("Test completed".into()))
                }
                path => {
                    // Get the value associated with the request URI, or return a 404 if it's not present
                    match store.get(path) {
                        Ok(value) => (StatusCode::OK, Some(value.into())),
                        Err(Error::NoSuchKey) => (StatusCode::NOT_FOUND, None),
                        Err(error) => return Err(error.into()),
                    }
                }
            }
        }
        Method::DELETE => {
            // Delete the value associated with the request URI, if present
            store.delete(req.uri().path())?;
            (StatusCode::OK, None)
        }
        _ => (StatusCode::METHOD_NOT_ALLOWED, None),
    };

    Ok(http::Response::builder().status(status).body(body)?)
}

// Tests a series of key/value operations within a single request to this component
fn full_test(store: &Store) -> Result<()> {
    test_opening_store(store)?;
    test_key_value_operations(store)?;
    Ok(())
}

fn test_key_value_operations(store: &Store) -> Result<()> {
    assert!(store.get_keys()?.is_empty());
    let kvs: Vec<(String, String)> = (0..10)
        .into_iter()
        .map(|i| (format!("k{i}"), format!("v{i}")))
        .collect();
    kvs.iter().for_each(|(k, v)| store.set(k, v).unwrap());
    kvs.iter()
        .for_each(|(k, v)| assert_eq!(&utf8s_to_string(&store.get(k).unwrap()), v));
    let stored_kvs = store.get_keys()?;
    assert_eq!(stored_kvs.len(), kvs.len());
    kvs.iter()
        .for_each(|(k, _)| assert!(stored_kvs.contains(k)));
    kvs.iter()
        .for_each(|(k, _)| store.set(k, "new value").unwrap());
    kvs.iter()
        .for_each(|(k, _)| assert_eq!(utf8s_to_string(&store.get(k).unwrap()), "new value"));
    kvs.iter().for_each(|(k, _)| store.delete(k).unwrap());
    assert!(store.get_keys()?.is_empty());
    Ok(())
}

fn test_opening_store(store: &Store) -> Result<()> {
    store.set("hello", "world")?;
    assert_eq!(utf8s_to_string(&store.get("hello")?), "world");
    // Open explicit "default" store
    let explicit_default_store = Store::open("default")?;
    // Check that is mapping to the same implicit store
    assert_eq!(
        utf8s_to_string(&explicit_default_store.get("hello")?),
        "world"
    );
    store.delete("hello")?;
    assert!(!store.exists("hello")?);
    // Ensure this component cannot open stores to which it has not been granted access
    match Store::open("not-permitted") {
        Err(Error::AccessDenied) => Ok(()),
        Err(e) => Err(anyhow::anyhow!(
            "Expected Error::AccessDenied but encountered {e:?}"
        )),
        Ok(_) => Err(anyhow::anyhow!("Tried to open a non-default store")),
    }
}

fn utf8s_to_string(v: &[u8]) -> String {
    std::str::from_utf8(v).unwrap().to_string()
}
