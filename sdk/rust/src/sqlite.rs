wit_bindgen_rust::import!("../../wit/ephemeral/sqlite.wit");

use sqlite::Connection as RawConnection;

/// Errors which may be raised by the methods of `Store`
pub type Error = sqlite::Error;

/// Represents a store in which key value tuples may be placed
#[derive(Debug)]
pub struct Connection(RawConnection);

impl Connection {
    /// Open a connection
    pub fn open() -> Result<Self, Error> {
        Ok(Self(sqlite::open("foo")?))
    }

    ///
    pub fn execute(&self, query: &str) -> Result<(), Error> {
        sqlite::execute(self.0, query)?;
        Ok(())
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for Error {}
