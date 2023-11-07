cargo_component_bindings::generate!();

use bindings::fermyon::spin::sqlite::{Connection, Error, Value};

struct Component;

#[macro_use]
mod handler;

fn main() -> Result<(), String> {
    ensure!(matches!(
        Connection::open("forbidden"),
        Err(Error::AccessDenied)
    ));

    let conn = r#try!(Connection::open("default"));

    let results = r#try!(conn.execute(
        "SELECT * FROM test_data WHERE key = ?",
        &[Value::Text("my_key".to_owned())],
    ));

    assert_eq!(1, results.rows.len());
    assert_eq!(2, results.columns.len());

    let key_index = results.columns.iter().position(|c| c == "key").unwrap();
    let value_index = results.columns.iter().position(|c| c == "value").unwrap();

    let fetched_key = &results.rows[0].values[key_index];
    let fetched_value = &results.rows[0].values[value_index];

    assert!(matches!(fetched_key, Value::Text(t) if t == "my_key"));
    assert!(matches!(fetched_value, Value::Text(t) if t == "my_value"));

    Ok(())
}
