use anyhow::{ensure, Result};
use spin_sdk::{
    sqlite::{Connection, Error, Value},
    wasi_http_component,
};

#[wasi_http_component]
fn handle_request(req: http::Request<()>) -> Result<http::Response<()>> {
    ensure!(matches!(
        Connection::open("forbidden"),
        Err(Error::AccessDenied)
    ));

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

    let conn = Connection::open_default()?;

    let results = conn.execute(
        "SELECT * FROM testdata WHERE key = ?",
        &[Value::Text(init_key.to_owned())],
    )?;

    assert_eq!(1, results.rows.len());
    assert_eq!(2, results.columns.len());

    let key_index = results.columns.iter().position(|c| c == "key").unwrap();
    let value_index = results.columns.iter().position(|c| c == "value").unwrap();

    let fetched_key: &str = results.rows[0].get(key_index).unwrap();
    let fetched_value: &str = results.rows[0].get(value_index).unwrap();

    assert_eq!(init_key, fetched_key);
    assert_eq!(init_val, fetched_value);

    Ok(http::Response::builder().status(200).body(())?)
}
