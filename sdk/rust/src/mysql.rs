//! Conversions between Rust, WIT and **MySQL** types.
//!
//! # Types
//!
//! | Rust type | WIT (db-value)      | MySQL type(s)           |
//! |-----------|---------------------|-------------------------|
//! | `bool`    | int8(s8)            | TINYINT(1), BOOLEAN     |
//! | `i8`      | int8(s8)            | TINYINT                 |
//! | `i16`     | int16(s16)          | SMALLINT                |
//! | `i32`     | int32(s32)          | MEDIUM, INT             |
//! | `i64`     | int64(s64)          | BIGINT                  |
//! | `u8`      | uint8(u8)           | TINYINT UNSIGNED        |
//! | `u16`     | uint16(u16)         | SMALLINT UNSIGNED       |
//! | `u32`     | uint32(u32)         | INT UNSIGNED            |
//! | `u64`     | uint64(u64)         | BIGINT UNSIGNED         |
//! | `f32`     | floating32(float32) | FLOAT                   |
//! | `f64`     | floating64(float64) | DOUBLE                  |
//! | `String`  | str(string)         | VARCHAR, CHAR, TEXT     |
//! | `Vec<u8>` | binary(list\<u8\>)  | VARBINARY, BINARY, BLOB |

/// Exports the generated outbound MySQL items.
pub use super::wit::mysql_types::*;
pub use super::wit::outbound_mysql::{execute, query};
pub use super::wit::rsbms_types::*;

impl std::error::Error for MysqlError {}

impl ::std::fmt::Display for MysqlError {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match self {
            MysqlError::ConnectionFailed(err_msg)
            | MysqlError::BadParameter(err_msg)
            | MysqlError::QueryFailed(err_msg)
            | MysqlError::ValueConversionFailed(err_msg)
            | MysqlError::OtherError(err_msg) => write!(f, "MySQL error: {}", err_msg),
            MysqlError::Success => panic!("Unexpected error: Success isn't supposed to be used"),
        }
    }
}

/// A MySQL error
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failed to deserialize [`DbValue`]
    #[error("error value decoding: {0}")]
    Decode(String),
    /// MySQL query failed with an error
    #[error(transparent)]
    MysqlError(#[from] MysqlError),
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
            DbValue::Int8(0) => Ok(false),
            DbValue::Int8(1) => Ok(true),
            _ => Err(Error::Decode(format_decode_err(
                "TINYINT(1), BOOLEAN",
                value,
            ))),
        }
    }
}

impl Decode for i8 {
    fn decode(value: &DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Int8(n) => Ok(*n),
            _ => Err(Error::Decode(format_decode_err("TINYINT", value))),
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

impl Decode for u8 {
    fn decode(value: &DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Uint8(n) => Ok(*n),
            _ => Err(Error::Decode(format_decode_err("UNSIGNED TINYINT", value))),
        }
    }
}

impl Decode for u16 {
    fn decode(value: &DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Uint16(n) => Ok(*n),
            _ => Err(Error::Decode(format_decode_err("UNSIGNED SMALLINT", value))),
        }
    }
}

impl Decode for u32 {
    fn decode(value: &DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Uint32(n) => Ok(*n),
            _ => Err(Error::Decode(format_decode_err(
                "UNISIGNED MEDIUMINT, UNSIGNED INT",
                value,
            ))),
        }
    }
}

impl Decode for u64 {
    fn decode(value: &DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Uint64(n) => Ok(*n),
            _ => Err(Error::Decode(format_decode_err("UNSIGNED BIGINT", value))),
        }
    }
}

impl Decode for f32 {
    fn decode(value: &DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Floating32(n) => Ok(*n),
            _ => Err(Error::Decode(format_decode_err("FLOAT", value))),
        }
    }
}

impl Decode for f64 {
    fn decode(value: &DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Floating64(n) => Ok(*n),
            _ => Err(Error::Decode(format_decode_err("DOUBLE", value))),
        }
    }
}

impl Decode for Vec<u8> {
    fn decode(value: &DbValue) -> Result<Self, Error> {
        match value {
            DbValue::Binary(n) => Ok(n.to_owned()),
            _ => Err(Error::Decode(format_decode_err("BINARY, VARBINARY", value))),
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
        assert!(bool::decode(&DbValue::Int8(1)).unwrap());
        assert!(bool::decode(&DbValue::Int8(3)).is_err());
        assert!(bool::decode(&DbValue::Int32(0)).is_err());
        assert!(Option::<bool>::decode(&DbValue::DbNull).unwrap().is_none());
    }

    #[test]
    fn int8() {
        assert_eq!(i8::decode(&DbValue::Int8(0)).unwrap(), 0);
        assert!(i8::decode(&DbValue::Int32(0)).is_err());
        assert!(Option::<i8>::decode(&DbValue::DbNull).unwrap().is_none());
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
    fn uint8() {
        assert_eq!(u8::decode(&DbValue::Uint8(0)).unwrap(), 0);
        assert!(u8::decode(&DbValue::Uint32(0)).is_err());
        assert!(Option::<u16>::decode(&DbValue::DbNull).unwrap().is_none());
    }

    #[test]
    fn uint16() {
        assert_eq!(u16::decode(&DbValue::Uint16(0)).unwrap(), 0);
        assert!(u16::decode(&DbValue::Uint32(0)).is_err());
        assert!(Option::<u16>::decode(&DbValue::DbNull).unwrap().is_none());
    }

    #[test]
    fn uint32() {
        assert_eq!(u32::decode(&DbValue::Uint32(0)).unwrap(), 0);
        assert!(u32::decode(&DbValue::Boolean(false)).is_err());
        assert!(Option::<u32>::decode(&DbValue::DbNull).unwrap().is_none());
    }

    #[test]
    fn uint64() {
        assert_eq!(u64::decode(&DbValue::Uint64(0)).unwrap(), 0);
        assert!(u64::decode(&DbValue::Boolean(false)).is_err());
        assert!(Option::<u64>::decode(&DbValue::DbNull).unwrap().is_none());
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
}
