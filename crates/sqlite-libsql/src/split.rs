use std::ffi::CStr;

use rusqlite::ffi;

/// Splits the given SQL into complete Sqlite statements.
///
/// Yields an error if the SQL includes incomplete Sqlite statements or if
/// Sqlite returns an error.
pub fn split_sql(mut sql: &str) -> impl Iterator<Item = Result<&str, Error>> {
    std::iter::from_fn(move || {
        if sql.is_empty() {
            return None;
        }
        match split_sql_once(sql) {
            Ok((stmt, tail)) => {
                sql = tail;
                Some(Ok(stmt))
            }
            Err(err) => {
                sql = "";
                Some(Err(err))
            }
        }
    })
}

/// Splits the given SQL into one complete Sqlite statement and any remaining
/// text after the ending semicolon.
///
/// Returns an error if the SQL is an _incomplete_ Sqlite statement or if Sqlite
/// returns an error.
pub fn split_sql_once(sql: &str) -> Result<(&str, &str), Error> {
    for (idx, _) in sql.match_indices(';') {
        let (candidate, tail) = sql.split_at(idx + 1);
        match ensure_complete(candidate) {
            Ok(()) => return Ok((candidate, tail)),
            Err(Error::Incomplete) => {
                // May be a semicolon inside e.g. a string literal.
                continue;
            }
            Err(err) => return Err(err),
        }
    }
    ensure_complete(sql)?;
    Ok((sql, ""))
}

// Validates that the given SQL is complete.
// Returns an error if the SQL is an incomplete Sqlite statement or if Sqlite
// returns an error.
fn ensure_complete(sql: &str) -> Result<(), Error> {
    let mut bytes: Vec<u8> = sql.into();
    if !bytes.ends_with(b";") {
        bytes.extend_from_slice(b"\n;");
    }
    bytes.push(b'\0');
    let c_str = CStr::from_bytes_with_nul(&bytes).unwrap();
    let c_ptr = c_str.as_ptr() as *const std::os::raw::c_char;
    match unsafe { ffi::sqlite3_complete(c_ptr) } {
        1 => Ok(()),
        0 => Err(Error::Incomplete),
        code => Err(Error::Sqlite(ffi::Error::new(code))),
    }
}

/// The error type for splitting SQL.
#[derive(Debug, PartialEq)]
pub enum Error {
    /// Returned for incomplete Sqlite statements, e.g. an unterminated string.
    Incomplete,
    /// Returned for errors from Sqlite itself.
    Sqlite(ffi::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Incomplete => write!(f, "not a complete SQL statement"),
            Self::Sqlite(err) => write!(f, "{err}"),
        }
    }
}
impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_sql() {
        for (input, want_stmts) in [
            ("", &[][..]),
            ("/* comment */", &["/* comment */"]),
            ("SELECT 1;", &["SELECT 1;"]),
            ("SELECT 1;SELECT 2", &["SELECT 1;", "SELECT 2"]),
            ("SELECT 1;SELECT 2", &["SELECT 1;", "SELECT 2"]),
        ] {
            let stmts = split_sql(input)
                .collect::<Result<Vec<_>, Error>>()
                .unwrap_or_else(|err| panic!("Failed to split {input:?}: {err}"));
            assert_eq!(stmts, want_stmts, "for {input:?}");
        }
    }

    #[test]
    fn test_split_sql_once_no_tail() {
        for input in [
            "",
            " ",
            "SELECT 1",
            "SELECT 1;",
            "SELECT * From some_table",
            "SELECT 1 -- trailing comment",
            "SELECT 1 -- trailing comment\n;",
            "SELECT 1 /* trailing comment */",
            "SELECT 1 /* trailing comment */;",
            "-- leading comment\nSELECT 1",
            "/* leading comment */ SELECT 1",
            "  -- Just a comment",
            "/* comment one */ -- comment two",
        ] {
            let (stmt, tail) = split_sql_once(input)
                .unwrap_or_else(|err| panic!("Failed to split {input:?}: {err}"));
            assert_eq!(stmt, input, "for {input:?}");
            assert_eq!(tail, "", "for {input:?}");
        }
    }

    #[test]
    fn test_split_sql_once_tail() {
        for (input, want_stmt, want_tail) in [
            ("SELECT 1; ", "SELECT 1;", " "),
            ("SELECT 1;SELECT 2", "SELECT 1;", "SELECT 2"),
            ("SELECT 1; -- tail", "SELECT 1;", " -- tail"),
            ("--leading\n; SELECT 1", "--leading\n;", " SELECT 1"),
            ("/* leading */; SELECT 1", "/* leading */;", " SELECT 1"),
        ] {
            let (stmt, tail) = split_sql_once(input)
                .unwrap_or_else(|err| panic!("Failed to split {input:?}: {err}"));
            assert_eq!(stmt, want_stmt, "for {input:?}");
            assert_eq!(tail, want_tail, "for {input:?}");
        }
    }

    #[test]
    fn test_split_sql_once_incomplete() {
        for input in [
            "SELECT 'incomplete string",
            "/* incomplete comment",
            "SELECT /* tricky comment '*/ '",
        ] {
            assert_eq!(
                split_sql_once(input),
                Err(Error::Incomplete),
                "for {input:?}"
            );
        }
    }
}
