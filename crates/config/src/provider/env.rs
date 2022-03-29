use anyhow::Context;

use crate::{Key, Provider};

const DEFAULT_PREFIX: &str = "SPIN_APP";

/// A config Provider that uses environment variables.
#[derive(Debug)]
pub struct EnvProvider {
    prefix: String,
}

impl EnvProvider {
    /// Creates a new EnvProvider.
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
        }
    }
}

impl Default for EnvProvider {
    fn default() -> Self {
        Self {
            prefix: DEFAULT_PREFIX.to_string(),
        }
    }
}

impl Provider for EnvProvider {
    fn get(&self, key: &Key) -> anyhow::Result<Option<String>> {
        let env_key = format!("{}_{}", &self.prefix, key.as_ref().to_ascii_uppercase());
        match std::env::var(&env_key) {
            Err(std::env::VarError::NotPresent) => Ok(None),
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
        let key = Key::new("env_key1").unwrap();
        assert_eq!(
            EnvProvider::new("TESTING_SPIN").get(&key).unwrap(),
            Some("val".to_string())
        );
    }

    #[test]
    fn provider_get_missing() {
        let key = Key::new("please_do_not_ever_set_this_during_tests").unwrap();
        assert_eq!(EnvProvider::default().get(&key).unwrap(), None);
    }
}
