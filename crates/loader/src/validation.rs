use anyhow::{anyhow, Result};

const ONLY_ALLOWED_KV_STORE: &str = "default";

pub(crate) fn validate_key_value_stores(key_value_stores: &Option<Vec<String>>) -> Result<()> {
    match key_value_stores
        .iter()
        .flatten()
        .find(|id| *id != ONLY_ALLOWED_KV_STORE)
    {
        None => Ok(()),
        Some(invalid) => {
            let err = anyhow!("Invalid key-value store '{invalid}'. This version of Spin supports only the '{ONLY_ALLOWED_KV_STORE}' store.");
            Err(err)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn kv_empty_list_is_allowed() {
        validate_key_value_stores(&None).expect("None should be valid");
        validate_key_value_stores(&Some(vec![])).expect("Empty vector should be valid");
    }

    #[test]
    fn default_store_is_allowed() {
        validate_key_value_stores(&Some(vec!["default".to_owned()]))
            .expect("Default store should be valid");
        validate_key_value_stores(&Some(vec!["default".to_owned(), "default".to_owned()]))
            .expect("Default store twice should be valid");
    }

    #[test]
    fn non_default_store_is_not_allowed() {
        validate_key_value_stores(&Some(vec!["hello".to_owned()]))
            .expect_err("'hello' store should be invalid");
    }

    #[test]
    fn no_sneaky_hiding_non_default_store_behind_default_one() {
        validate_key_value_stores(&Some(vec!["default".to_owned(), "hello".to_owned()]))
            .expect_err("'hello' store should be invalid");
    }
}
