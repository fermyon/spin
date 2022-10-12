
wit_bindgen_rust::import!("../../wit/ephemeral/outbound-pg.wit");

/// Exports the generated outbound Pg items.
pub use outbound_pg::*;

impl TryFrom<&DbValue> for i32 {
    type Error = anyhow::Error;

    fn try_from(value: &DbValue) -> Result<Self, Self::Error> {
        match value {
            DbValue::Int32(n) => Ok(*n),
            _ => Err(anyhow::anyhow!(
                "Expected integer from database but got {:?}",
                value
            )),
        }
    }
}

impl TryFrom<&DbValue> for String {
    type Error = anyhow::Error;

    fn try_from(value: &DbValue) -> Result<Self, Self::Error> {
        match value {
            DbValue::Str(s) => Ok(s.to_owned()),
            _ => Err(anyhow::anyhow!(
                "Expected string from the DB but got {:?}",
                value
            )),
        }
    }
}

impl TryFrom<&DbValue> for i64 {
    type Error = anyhow::Error;

    fn try_from(value: &DbValue) -> Result<Self, Self::Error> {
        match value {
            DbValue::Int64(n) => Ok(*n),
            _ => Err(anyhow::anyhow!(
                "Expected integer from the DB but got {:?}",
                value
            )),
        }
    }
}
