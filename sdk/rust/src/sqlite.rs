#![allow(missing_docs)]

use super::wit::fermyon::spin::sqlite;
use sqlite::Connection as RawConnection;

/// Errors which may be raised by the methods of `Store`
pub use sqlite::Error;
/// The result of making a query
pub use sqlite::QueryResult;
/// A row in a QueryResult
pub use sqlite::RowResult;
/// A parameter used when executing a sqlite statement
pub use sqlite::ValueParam;
/// A single column's result from a database query
pub use sqlite::ValueResult;

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
    pub fn execute(
        &self,
        query: &str,
        parameters: &[ValueParam<'_>],
    ) -> Result<sqlite::QueryResult, Error> {
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
    pub fn get<T: TryFrom<&'a ValueResult>>(&self, column: &str) -> Option<T> {
        let i = self.columns.iter().position(|c| c == column)?;
        self.result.get(i)
    }
}

impl sqlite::RowResult {
    /// Get a value by its index
    pub fn get<'a, T: TryFrom<&'a ValueResult>>(&'a self, index: usize) -> Option<T> {
        self.values.get(index).and_then(|c| c.try_into().ok())
    }
}

impl<'a> TryFrom<&'a ValueResult> for bool {
    type Error = ();

    fn try_from(value: &'a ValueResult) -> Result<Self, Self::Error> {
        match value {
            ValueResult::Integer(i) => Ok(*i != 0),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<&'a ValueResult> for u32 {
    type Error = ();

    fn try_from(value: &'a ValueResult) -> Result<Self, Self::Error> {
        match value {
            ValueResult::Integer(i) => Ok(*i as u32),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<&'a ValueResult> for &'a str {
    type Error = ();

    fn try_from(value: &'a ValueResult) -> Result<Self, Self::Error> {
        match value {
            ValueResult::Text(s) => Ok(s.as_str()),
            _ => Err(()),
        }
    }
}
