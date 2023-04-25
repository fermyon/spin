//! Conversions between Rust, WIT and **Postgres** types.
//!
//! # Types
//!
//! | Rust type  | WIT (db-value)      | Postgres type(s)             |
//! |------------|---------------------|----------------------------- |
//! | `bool`     | boolean(bool)       | BOOL                         |
//! | `i16`      | int16(s16)          | SMALLINT, SMALLSERIAL, INT2  |
//! | `i32`      | int32(s32)          | INT, SERIAL, INT4            |
//! | `i64`      | int64(s64)          | BIGINT, BIGSERIAL, INT8      |
//! | `f32`      | floating32(float32) | REAL, FLOAT4                 |
//! | `f64`      | floating64(float64) | DOUBLE PRECISION, FLOAT8     |
//! | `String`   | str(string)         | VARCHAR, CHAR(N), TEXT       |
//! | `Vec<u8>`  | binary(list\<u8\>)  | BYTEA                        |

pub use super::wit::outbound_pg::{execute, query};
/// Exports the generated outbound Pg items.
pub use super::wit::pg_types::*;
pub use super::wit::rsbms_types::*;

impl std::error::Error for PgError {}

impl ::std::fmt::Display for PgError {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match self {
            PgError::ConnectionFailed(err_msg)
            | PgError::BadParameter(err_msg)
            | PgError::QueryFailed(err_msg)
            | PgError::ValueConversionFailed(err_msg)
            | PgError::OtherError(err_msg) => write!(f, "Postgres error: {}", err_msg),
            PgError::Success => panic!("Unexpected error: Success isn't supposed to be used"),
        }
    }
}

/// A pg error
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failed to deserialize [`DbValue`]
    #[error("error value decoding: {0}")]
    Decode(String),
    /// Pg query failed with an error
    #[error("{0}")]
    PgError(#[from] PgError),
}

/// A type that can be decoded from the database.
pub trait Decode: Sized {
    /// Decode a new value of this type using a [`DbValue`].
    fn decode(value: &DbValue) -> Result<Self, Error>;
}

impl<T> Decode for Option<T>
where
    T: Decode,
{
    fn decode(value: &DbValue) -> Result<Self, Error> {
        match value {
            DbValue::DbNull => Ok(None),
            v => Ok(Some(T::decode(v)?)),
        }
    }
}

impl Decode for bool {
    fn decode(value: &DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Boolean(boolean) => Ok(*boolean),
            _ => Err(Error::Decode(format_decode_err("BOOL", value))),
        }
    }
}

impl Decode for i16 {
    fn decode(value: &DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Int16(n) => Ok(*n),
            _ => Err(Error::Decode(format_decode_err("SMALLINT", value))),
        }
    }
}

impl Decode for i32 {
    fn decode(value: &DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Int32(n) => Ok(*n),
            _ => Err(Error::Decode(format_decode_err("INT", value))),
        }
    }
}

impl Decode for i64 {
    fn decode(value: &DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Int64(n) => Ok(*n),
            _ => Err(Error::Decode(format_decode_err("BIGINT", value))),
        }
    }
}

impl Decode for f32 {
    fn decode(value: &DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Floating32(n) => Ok(*n),
            _ => Err(Error::Decode(format_decode_err("REAL", value))),
        }
    }
}

impl Decode for f64 {
    fn decode(value: &DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Floating64(n) => Ok(*n),
            _ => Err(Error::Decode(format_decode_err("DOUBLE PRECISION", value))),
        }
    }
}

impl Decode for Vec<u8> {
    fn decode(value: &DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Binary(n) => Ok(n.to_owned()),
            _ => Err(Error::Decode(format_decode_err("BYTEA", value))),
        }
    }
}

impl Decode for String {
    fn decode(value: &DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Str(s) => Ok(s.to_owned()),
            _ => Err(Error::Decode(format_decode_err(
                "CHAR, VARCHAR, TEXT",
                value,
            ))),
        }
    }
}

fn format_decode_err(types: &str, value: &DbValue) -> String {
    format!("Expected {} from the DB but got {:?}", types, value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boolean() {
        assert!(bool::decode(&DbValue::Boolean(true)).unwrap());
        assert!(bool::decode(&DbValue::Int32(0)).is_err());
        assert!(Option::<bool>::decode(&DbValue::DbNull).unwrap().is_none());
    }

    #[test]
    fn int16() {
        assert_eq!(i16::decode(&DbValue::Int16(0)).unwrap(), 0);
        assert!(i16::decode(&DbValue::Int32(0)).is_err());
        assert!(Option::<i16>::decode(&DbValue::DbNull).unwrap().is_none());
    }

    #[test]
    fn int32() {
        assert_eq!(i32::decode(&DbValue::Int32(0)).unwrap(), 0);
        assert!(i32::decode(&DbValue::Boolean(false)).is_err());
        assert!(Option::<i32>::decode(&DbValue::DbNull).unwrap().is_none());
    }

    #[test]
    fn int64() {
        assert_eq!(i64::decode(&DbValue::Int64(0)).unwrap(), 0);
        assert!(i64::decode(&DbValue::Boolean(false)).is_err());
        assert!(Option::<i64>::decode(&DbValue::DbNull).unwrap().is_none());
    }

    #[test]
    fn floating32() {
        assert!(f32::decode(&DbValue::Floating32(0.0)).is_ok());
        assert!(f32::decode(&DbValue::Boolean(false)).is_err());
        assert!(Option::<f32>::decode(&DbValue::DbNull).unwrap().is_none());
    }

    #[test]
    fn floating64() {
        assert!(f64::decode(&DbValue::Floating64(0.0)).is_ok());
        assert!(f64::decode(&DbValue::Boolean(false)).is_err());
        assert!(Option::<f64>::decode(&DbValue::DbNull).unwrap().is_none());
    }

    #[test]
    fn str() {
        assert_eq!(
            String::decode(&DbValue::Str(String::from("foo"))).unwrap(),
            String::from("foo")
        );

        assert!(String::decode(&DbValue::Int32(0)).is_err());
        assert!(Option::<String>::decode(&DbValue::DbNull)
            .unwrap()
            .is_none());
    }

    #[test]
    fn binary() {
        assert!(Vec::<u8>::decode(&DbValue::Binary(vec![0, 0])).is_ok());
        assert!(Vec::<u8>::decode(&DbValue::Boolean(false)).is_err());
        assert!(Option::<Vec<u8>>::decode(&DbValue::DbNull)
            .unwrap()
            .is_none());
    }
}
