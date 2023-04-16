wit_bindgen_rust::import!("../../wit/ephemeral/sqlite.wit");

use sqlite::Connection as RawConnection;

/// Errors which may be raised by the methods of `Store`
pub type Error = sqlite::Error;

///
pub type Row = sqlite::Row;

///
pub type DataTypeParam<'a> = sqlite::DataTypeParam<'a>;
///
pub type DataTypeResult = sqlite::DataTypeResult;

/// Represents a store in which key value tuples may be placed
#[derive(Debug)]
pub struct Connection(RawConnection);

impl Connection {
    /// Open a connection
    pub fn open() -> Result<Self, Error> {
        Ok(Self(sqlite::open("foo")?))
    }

    /// Make a query against the database
    pub fn query(&self, statement: &Statement) -> Result<Vec<sqlite::Row>, Error> {
        sqlite::query(self.0, statement.0)
    }

    /// Execute a statement against the database
    pub fn execute(&self, statement: &str) -> Result<(), Error> {
        let statement = Statement::prepare(statement, &[])?;
        sqlite::execute(self.0, statement.0)?;
        Ok(())
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for Error {}

/// A prepared statement
pub struct Statement(sqlite::Statement);

impl Statement {
    /// Prepare a statement
    pub fn prepare(query: &str, params: &[DataTypeParam]) -> Result<Statement, sqlite::Error> {
        let statement = sqlite::prepare_statement(query, params)?;
        Ok(Statement(statement))
    }
}

impl Drop for Statement {
    fn drop(&mut self) {
        sqlite::drop_statement(self.0);
    }
}
