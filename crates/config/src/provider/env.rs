use std::collections::HashMap;

use anyhow::Context;

use crate::{Key, Provider};

pub const DEFAULT_PREFIX: &str = "SPIN_APP";

/// A config Provider that uses environment variables.
#[derive(Debug)]
pub struct EnvProvider {
    prefix: String,
    envs: HashMap<String, String>,
}

impl EnvProvider {
    /// Creates a new EnvProvider.
    pub fn new(prefix: impl Into<String>, envs: HashMap<String, String>) -> Self {
        Self {
            prefix: prefix.into(),
            envs,
        }
    }
}

impl Default for EnvProvider {
    fn default() -> Self {
        Self {
            prefix: DEFAULT_PREFIX.to_string(),
            envs: HashMap::new(),
        }
    }
}

impl Provider for EnvProvider {
    fn get(&self, key: &Key) -> anyhow::Result<Option<String>> {
        let env_key = format!("{}_{}", &self.prefix, key.as_ref().to_ascii_uppercase());
        match std::env::var(&env_key) {
            Err(std::env::VarError::NotPresent) => {
                if let Some(value) = self.envs.get(&env_key) {
                    return Ok(Some(value.to_string()));
                }

                Ok(None)
            }
            other => other
                .map(Some)
                .with_context(|| format!("failed to resolve env var {}", &env_key)),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn provider_get() {
        std::env::set_var("TESTING_SPIN_ENV_KEY1", "val");
        let key1 = Key::new("env_key1").unwrap();
        let mut envs = HashMap::new();
        envs.insert(
            "TESTING_SPIN_ENV_KEY1".to_string(),
            "dotenv_val".to_string(),
        );
        assert_eq!(
            EnvProvider::new("TESTING_SPIN", envs.clone())
                .get(&key1)
                .unwrap(),
            Some("val".to_string())
        );

        let key2 = Key::new("env_key2").unwrap();
        envs.insert(
            "TESTING_SPIN_ENV_KEY2".to_string(),
            "dotenv_val".to_string(),
        );
        assert_eq!(
            EnvProvider::new("TESTING_SPIN", envs.clone())
                .get(&key2)
                .unwrap(),
            Some("dotenv_val".to_string())
        );
    }

    #[test]
    fn provider_get_missing() {
        let key = Key::new("please_do_not_ever_set_this_during_tests").unwrap();
        assert_eq!(EnvProvider::default().get(&key).unwrap(), None);
    }
}
