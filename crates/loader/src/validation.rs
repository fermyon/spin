use std::collections::HashMap;

use anyhow::{ensure, Context, Result};

use crate::common::RawVariable;

pub(crate) fn validate_variable_names(variables: &HashMap<String, RawVariable>) -> Result<()> {
    for name in variables.keys() {
        if let Err(spin_config::Error::InvalidKey(m)) = spin_config::Key::new(name) {
            anyhow::bail!("Invalid variable name {name}: {m}. Variable names and config keys may contain only lower-case letters, numbers, and underscores.");
        };
    }
    Ok(())
}

pub(crate) fn validate_config_keys(config: &Option<HashMap<String, String>>) -> Result<()> {
    for name in config.iter().flat_map(|c| c.keys()) {
        if let Err(spin_config::Error::InvalidKey(m)) = spin_config::Key::new(name) {
            anyhow::bail!("Invalid config key {m}"); // No need to give name as it's already in the message
        };
    }
    Ok(())
}

pub(crate) fn validate_key_value_stores(key_value_stores: &Option<Vec<String>>) -> Result<()> {
    for store in key_value_stores.iter().flatten() {
        validate_component_like_label(store)
            .with_context(|| format!("invalid store label {store:?}"))?;
    }
    Ok(())
}

// For forward-compatibility with component model value imports, validate that
// the given string is like a component model label, except (currently) with
// snake_case instead of kebab-case.
fn validate_component_like_label(label: &str) -> Result<()> {
    ensure!(!label.is_empty(), "label may not be empty");
    for word in label.split('_') {
        ensure!(
            word.chars().all(|c| c.is_ascii_alphanumeric()),
            "labels may contain only ascii alphanumeric words separated by underscores"
        );
        let initial = word
            .chars()
            .next()
            .context("label words may not be empty")?;
        ensure!(
            initial.is_ascii_alphabetic(),
            "label words must start with an ascii letter"
        );
        let is_upper = initial.is_ascii_uppercase();
        ensure!(
            word.chars().all(|c| c.is_ascii_uppercase() == is_upper),
            "label words must be all lowercase or all uppercase"
        );
    }
    Ok(())
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
    fn valid_store_names_are_allowed() -> Result<()> {
        for valid_name in ["default", "mixed_CASE_words", "letters1_then2_numbers345"] {
            validate_key_value_stores(&Some(vec![valid_name.to_string()]))
                .with_context(|| format!("{valid_name:?} should be valid"))?;
        }
        Ok(())
    }

    #[test]
    fn invalid_store_names_are_rejected() -> Result<()> {
        for invalid_name in [
            "",
            "kebab-case",
            "_leading_underscore",
            "trailing_underscore_",
            "double__underscore",
            "1initial_number",
            "unicode_snowpeople☃☃☃",
            "mIxEd_case",
            "MiXeD_case",
        ] {
            validate_key_value_stores(&Some(vec![invalid_name.to_string()]))
                .err()
                .with_context(|| format!("{invalid_name:?} should be invalid"))?;
        }
        Ok(())
    }
}
