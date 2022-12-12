use anyhow::Result;
use spin_sdk::mysql::{self, Decode};

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
    let id = i32::decode(&row[0])?;
    let name = String::decode(&row[1])?;
    let prey = Option::<String>::decode(&row[2])?;
    let is_finicky = bool::decode(&row[3])?;

    Ok(Pet {
        id,
        name,
        prey,
        is_finicky,
    })
}
