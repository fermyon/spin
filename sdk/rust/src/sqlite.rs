wit_bindgen_rust::import!("../../wit/ephemeral/sqlite.wit");

use sqlite::Connection as RawConnection;

/// Errors which may be raised by the methods of `Store`
pub type Error = sqlite::Error;

/// A parameter used when executing a sqlite statement
pub type ValueParam<'a> = sqlite::ValueParam<'a>;
/// A single column's result from a database query
pub type ValueResult = sqlite::ValueResult;

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
    ) -> Result<QueryResult, Error> {
        let inner = sqlite::execute(self.0, query, parameters)?;
        let columns = sqlite::get_columns(inner)?;
        Ok(QueryResult { inner, columns })
    }
}

/// The data returned from querying the database
pub struct QueryResult {
    inner: sqlite::QueryResult,
    columns: Vec<String>,
}

impl QueryResult {
    /// Get a specific row for this query result
    pub fn row(&self, index: usize) -> Result<Option<RowResult<'_>>, sqlite::Error> {
        let row_result = sqlite::get_row_result(self.inner, index as u32)?;
        Ok(row_result.map(|r| RowResult {
            columns: self.columns.as_slice(),
            result: r,
        }))
    }

    /// Get all the rows for this query result
    pub fn rows(&self) -> impl Iterator<Item = Result<RowResult<'_>, sqlite::Error>> {
        let mut index = 0;
        std::iter::from_fn(move || {
            let r = self.row(index).transpose()?;
            index += 1;
            Some(r)
        })
    }
}

impl Drop for QueryResult {
    fn drop(&mut self) {
        sqlite::free_query_result(self.inner);
    }
}

/// A database row result
pub struct RowResult<'a> {
    columns: &'a [String],
    result: sqlite::RowResult,
}

impl<'a> RowResult<'a> {
    /// Get a value by its column name
    pub fn get<T: TryFrom<&'a ValueResult>>(&'a self, column: &str) -> Option<T> {
        let i = self.columns.iter().position(|c| c == column)?;
        self.result.get(i)
    }

    /// Get all the values for this row result
    pub fn get_at<T: TryFrom<&'a ValueResult>>(&'a self, index: usize) -> Option<T> {
        self.result.get(index)
    }
}

impl sqlite::RowResult {
    fn get<'a, T: TryFrom<&'a ValueResult>>(&'a self, index: usize) -> Option<T> {
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

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for Error {}
