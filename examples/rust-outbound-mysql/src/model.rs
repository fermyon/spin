use crate::convert::{as_i8_bool, as_int, as_owned_string, as_owned_string_opt};
use anyhow::Result;
use spin_sdk::mysql::{self};

// Such logic, very business

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct Pet {
    id: i32,
    name: String,
    prey: Option<String>,
    is_finicky: bool,
}

pub(crate) fn as_pet(row: &mysql::Row) -> Result<Pet> {
    let id = as_int(&row[0])?;
    let name = as_owned_string(&row[1])?;
    let prey = as_owned_string_opt(&row[2])?;
    let is_finicky = as_i8_bool(&row[3])?;

    Ok(Pet {
        id,
        name,
        prey,
        is_finicky,
    })
}
