use helper::{ensure_eq, ensure_matches, ensure_ok, ensure_some};

use bindings::fermyon::spin2_0_0::sqlite::{Connection, Error, Value};

helper::define_component!(Component);

impl Component {
    fn main() -> Result<(), String> {
        ensure_matches!(Connection::open("forbidden"), Err(Error::AccessDenied));

        let conn = ensure_ok!(Connection::open("default"));
        ensure_ok!(conn.execute(
            "CREATE TABLE IF NOT EXISTS test_data(key TEXT NOT NULL, value TEXT NOT NULL);",
            &[]
        ));
        ensure_ok!(conn.execute(
            "INSERT INTO test_data(key, value) VALUES('my_key', 'my_value');",
            &[]
        ));

        let results = ensure_ok!(conn.execute(
            "SELECT * FROM test_data WHERE key = ?",
            &[Value::Text("my_key".to_owned())],
        ));

        ensure_eq!(1, results.rows.len());
        ensure_eq!(2, results.columns.len());

        let key_index = ensure_some!(results.columns.iter().position(|c| c == "key"));
        let value_index = ensure_some!(results.columns.iter().position(|c| c == "value"));

        let fetched_key = &results.rows[0].values[key_index];
        let fetched_value = &results.rows[0].values[value_index];

        ensure_matches!(fetched_key, Value::Text(t) if t == "my_key");
        ensure_matches!(fetched_value, Value::Text(t) if t == "my_value");

        Ok(())
    }
}
