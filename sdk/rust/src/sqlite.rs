#![allow(missing_docs)]

use super::wit::fermyon::spin::sqlite;
use sqlite::Connection as RawConnection;

/// Errors which may be raised by the methods of `Store`
pub use sqlite::Error;
/// The result of making a query
pub use sqlite::QueryResult;
/// A row in a QueryResult
pub use sqlite::RowResult;
/// A single column's result from a database query
pub use sqlite::Value;

/// Represents a store in which key value tuples may be placed
#[derive(Debug)]
pub struct Connection(RawConnection);

impl Connection {
    /// Open a connection to the default database
    pub fn open_default() -> Result<Self, Error> {
        Ok(Self(sqlite::open("default")?))
    }

    /// Open a connection
    pub fn open(database: &str) -> Result<Self, Error> {
        Ok(Self(sqlite::open(database)?))
    }

    /// Execute a statement against the database
    pub fn execute(&self, query: &str, parameters: &[Value]) -> Result<sqlite::QueryResult, Error> {
        sqlite::execute(self.0, query, parameters)
    }
}

impl sqlite::QueryResult {
    /// Get all the rows for this query result
    pub fn rows(&self) -> impl Iterator<Item = Row<'_>> {
        self.rows.iter().map(|r| Row {
            columns: self.columns.as_slice(),
            result: r,
        })
    }
}

/// A database row result
pub struct Row<'a> {
    columns: &'a [String],
    result: &'a sqlite::RowResult,
}

impl<'a> Row<'a> {
    /// Get a value by its column name
    pub fn get<T: TryFrom<&'a Value>>(&self, column: &str) -> Option<T> {
        let i = self.columns.iter().position(|c| c == column)?;
        self.result.get(i)
    }
}

impl sqlite::RowResult {
    /// Get a value by its index
    pub fn get<'a, T: TryFrom<&'a Value>>(&'a self, index: usize) -> Option<T> {
        self.values.get(index).and_then(|c| c.try_into().ok())
    }
}

impl<'a> TryFrom<&'a Value> for bool {
    type Error = ();

    fn try_from(value: &'a Value) -> Result<Self, Self::Error> {
        match value {
            Value::Integer(i) => Ok(*i != 0),
            _ => Err(()),
        }
    }
}

macro_rules! int_conversions {
    ($($t:ty),*) => {
        $(impl<'a> TryFrom<&'a Value> for $t {
            type Error = ();

            fn try_from(value: &'a Value) -> Result<Self, Self::Error> {
                match value {
                    Value::Integer(i) => (*i).try_into().map_err(|_| ()),
                    _ => Err(()),
                }
            }
        })*
    };
}

int_conversions!(u8, u16, u32, u64, i8, i16, i32, i64, usize, isize);

impl<'a> TryFrom<&'a Value> for f64 {
    type Error = ();

    fn try_from(value: &'a Value) -> Result<Self, Self::Error> {
        match value {
            Value::Real(f) => Ok(*f),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<&'a Value> for &'a str {
    type Error = ();

    fn try_from(value: &'a Value) -> Result<Self, Self::Error> {
        match value {
            Value::Text(s) => Ok(s.as_str()),
            Value::Blob(b) => std::str::from_utf8(b).map_err(|_| ()),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<&'a Value> for &'a [u8] {
    type Error = ();

    fn try_from(value: &'a Value) -> Result<Self, Self::Error> {
        match value {
            Value::Blob(b) => Ok(b.as_slice()),
            Value::Text(s) => Ok(s.as_bytes()),
            _ => Err(()),
        }
    }
}
