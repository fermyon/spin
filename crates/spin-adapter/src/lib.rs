cargo_component_bindings::generate!();

use std::mem::ManuallyDrop;

use bindings::exports::fermyon::spin_one::{key_value, sqlite};

use bindings::fermyon::spin_two;

struct Component;

impl From<spin_two::key_value::Error> for key_value::Error {
    fn from(value: spin_two::key_value::Error) -> Self {
        match value {
            spin_two::key_value::Error::StoreTableFull => Self::StoreTableFull,
            spin_two::key_value::Error::NoSuchStore => Self::NoSuchStore,
            spin_two::key_value::Error::AccessDenied => Self::AccessDenied,
            spin_two::key_value::Error::InvalidStore => Self::InvalidStore,
            spin_two::key_value::Error::NoSuchKey => Self::NoSuchKey,
            spin_two::key_value::Error::Io(s) => Self::Io(s),
        }
    }
}

impl key_value::Guest for Component {
    fn open(name: String) -> Result<key_value::Store, key_value::Error> {
        let store =
            spin_two::key_value::Store::open(&name).map_err::<key_value::Error, _>(From::from)?;
        let handle = store.handle();
        // Forget the store so that we don't run the destructor
        std::mem::forget(store);
        Ok(handle)
    }

    fn get(store: key_value::Store, key: String) -> Result<Vec<u8>, key_value::Error> {
        let store = ManuallyDrop::new(unsafe { spin_two::key_value::Store::from_handle(store) });
        store.get(&key).map_err::<key_value::Error, _>(From::from)
    }

    fn set(store: key_value::Store, key: String, value: Vec<u8>) -> Result<(), key_value::Error> {
        let store = ManuallyDrop::new(unsafe { spin_two::key_value::Store::from_handle(store) });
        store
            .set(&key, &value)
            .map_err::<key_value::Error, _>(From::from)
    }

    fn delete(store: key_value::Store, key: String) -> Result<(), key_value::Error> {
        let store = ManuallyDrop::new(unsafe { spin_two::key_value::Store::from_handle(store) });
        store
            .delete(&key)
            .map_err::<key_value::Error, _>(From::from)
    }

    fn exists(store: key_value::Store, key: String) -> Result<bool, key_value::Error> {
        let store = ManuallyDrop::new(unsafe { spin_two::key_value::Store::from_handle(store) });
        store
            .exists(&key)
            .map_err::<key_value::Error, _>(From::from)
    }

    fn get_keys(store: key_value::Store) -> Result<Vec<String>, key_value::Error> {
        let store = ManuallyDrop::new(unsafe { spin_two::key_value::Store::from_handle(store) });
        store.get_keys().map_err::<key_value::Error, _>(From::from)
    }

    fn close(store: key_value::Store) {
        // Run the destructor
        let _ = unsafe { spin_two::key_value::Store::from_handle(store) };
    }
}

impl From<spin_two::sqlite::Error> for sqlite::Error {
    fn from(value: spin_two::sqlite::Error) -> Self {
        match value {
            spin_two::sqlite::Error::NoSuchDatabase => Self::NoSuchDatabase,
            spin_two::sqlite::Error::AccessDenied => Self::AccessDenied,
            spin_two::sqlite::Error::InvalidConnection => Self::InvalidConnection,
            spin_two::sqlite::Error::DatabaseFull => Self::DatabaseFull,
            spin_two::sqlite::Error::Io(s) => Self::Io(s),
        }
    }
}

impl From<sqlite::Value> for spin_two::sqlite::Value {
    fn from(value: sqlite::Value) -> Self {
        match value {
            sqlite::Value::Integer(i) => Self::Integer(i),
            sqlite::Value::Real(r) => Self::Real(r),
            sqlite::Value::Text(t) => Self::Text(t),
            sqlite::Value::Blob(b) => Self::Blob(b),
            sqlite::Value::Null => Self::Null,
        }
    }
}

impl From<spin_two::sqlite::QueryResult> for sqlite::QueryResult {
    fn from(query_result: spin_two::sqlite::QueryResult) -> Self {
        Self {
            columns: query_result.columns,
            rows: query_result.rows.into_iter().map(From::from).collect(),
        }
    }
}

impl From<spin_two::sqlite::RowResult> for sqlite::RowResult {
    fn from(row_result: spin_two::sqlite::RowResult) -> Self {
        Self {
            values: row_result.values.into_iter().map(From::from).collect(),
        }
    }
}

impl From<spin_two::sqlite::Value> for sqlite::Value {
    fn from(value: spin_two::sqlite::Value) -> Self {
        match value {
            spin_two::sqlite::Value::Integer(i) => Self::Integer(i),
            spin_two::sqlite::Value::Real(r) => Self::Real(r),
            spin_two::sqlite::Value::Text(t) => Self::Text(t),
            spin_two::sqlite::Value::Blob(b) => Self::Blob(b),
            spin_two::sqlite::Value::Null => Self::Null,
        }
    }
}

impl sqlite::Guest for Component {
    fn open(database: String) -> Result<sqlite::Connection, sqlite::Error> {
        let connection = spin_two::sqlite::Connection::open(&database)
            .map_err::<sqlite::Error, _>(From::from)?;
        let handle = connection.handle();
        // Forget the connection so that we don't run the destructor
        std::mem::forget(connection);
        Ok(handle)
    }

    fn execute(
        conn: sqlite::Connection,
        statement: String,
        parameters: Vec<sqlite::Value>,
    ) -> Result<sqlite::QueryResult, sqlite::Error> {
        let conn =
            std::mem::ManuallyDrop::new(unsafe { spin_two::sqlite::Connection::from_handle(conn) });
        let result = conn
            .execute(
                &statement,
                &parameters.into_iter().map(From::from).collect::<Vec<_>>(),
            )
            .map_err::<sqlite::Error, _>(From::from)?;
        Ok(result.into())
    }

    fn close(conn: sqlite::Connection) {
        // Run the destructor
        let _ = unsafe { spin_two::sqlite::Connection::from_handle(conn) };
    }
}
