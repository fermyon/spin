wit_bindgen_rust::import!("../../wit/ephemeral/sqlite.wit");

use sqlite::Connection as RawConnection;

/// Errors which may be raised by the methods of `Store`
pub type Error = sqlite::Error;

///
pub type Row = sqlite::Row;

///
pub type DataTypeParam<'a> = sqlite::ValueParam<'a>;
///
pub type DataTypeResult = sqlite::ValueResult;

/// Represents a store in which key value tuples may be placed
#[derive(Debug)]
pub struct Connection(RawConnection);

impl Connection {
    /// Open a connection
    pub fn open() -> Result<Self, Error> {
        Ok(Self(sqlite::open("foo")?))
    }

    /// Execute a statement against the database
    pub fn execute<'a>(
        &self,
        statement: &str,
        parameters: &[sqlite::ValueParam<'a>],
    ) -> Result<(), Error> {
        sqlite::execute(self.0, statement, parameters)?;
        Ok(())
    }

    /// Make a query against the database
    pub fn query<'a>(
        &self,
        query: &str,
        parameters: &[DataTypeParam<'a>],
    ) -> Result<Vec<sqlite::Row>, Error> {
        sqlite::query(self.0, query, parameters)
    }
}

impl Row {
    pub fn get<'a, T: TryFrom<&'a sqlite::ValueResult>>(&'a self, name: &str) -> Option<T> {
        self.values
            .iter()
            .find_map(|c| (c.name == name).then(|| (&c.value).try_into().ok()))
            .flatten()
    }

    pub fn geti<'a, T: TryFrom<&'a sqlite::ValueResult>>(&'a self, index: usize) -> Option<T> {
        self.values
            .get(index)
            .map(|c| (&c.value).try_into().ok())
            .flatten()
    }
}

impl<'a> TryFrom<&'a sqlite::ValueResult> for bool {
    type Error = ();

    fn try_from(value: &'a sqlite::ValueResult) -> Result<Self, Self::Error> {
        match value {
            sqlite::ValueResult::Integer(i) => Ok(*i != 0),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<&'a sqlite::ValueResult> for u32 {
    type Error = ();

    fn try_from(value: &'a sqlite::ValueResult) -> Result<Self, Self::Error> {
        match value {
            sqlite::ValueResult::Integer(i) => Ok(*i as u32),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<&'a sqlite::ValueResult> for &'a str {
    type Error = ();

    fn try_from(value: &'a sqlite::ValueResult) -> Result<Self, Self::Error> {
        match value {
            sqlite::ValueResult::Text(s) => Ok(s.as_str()),
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
