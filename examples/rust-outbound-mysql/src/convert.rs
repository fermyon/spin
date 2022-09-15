use anyhow::{anyhow, Result};
use spin_sdk::mysql::{self};

pub(crate) fn as_owned_string(value: &mysql::DbValue) -> Result<String> {
    match value {
        mysql::DbValue::Str(s) => Ok(s.to_owned()),
        _ => Err(anyhow!("Expected string from database but got {:?}", value)),
    }
}

pub(crate) fn as_owned_string_opt(value: &mysql::DbValue) -> Result<Option<String>> {
    match value {
        mysql::DbValue::Str(s) => Ok(Some(s.to_owned())),
        mysql::DbValue::DbNull => Ok(None),
        _ => Err(anyhow!(
            "Expected string or null from database but got {:?}",
            value
        )),
    }
}

pub(crate) fn as_int(value: &mysql::DbValue) -> Result<i32> {
    match value {
        mysql::DbValue::Int32(n) => Ok(*n),
        _ => Err(anyhow!(
            "Expected integer from database but got {:?}",
            value
        )),
    }
}

pub(crate) fn as_i8_bool(value: &mysql::DbValue) -> Result<bool> {
    // MySQL doesn't have a distinct bool type - BOOL is actually TINYINT and
    // surfaces as such
    match value {
        mysql::DbValue::Int8(n) => Ok(*n != 0),
        _ => Err(anyhow!(
            "Expected boolean from database but got {:?}",
            value
        )),
    }
}

pub(crate) fn to_i8_bool(value: bool) -> i8 {
    if value {
        1
    } else {
        0
    }
}
