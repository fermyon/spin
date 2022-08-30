use std::{collections::HashMap, path::PathBuf, sync::Mutex};

use anyhow::{Context, Result};
use async_trait::async_trait;

use crate::{Key, Provider};

/// A config Provider that uses environment variables.
#[derive(Debug)]
pub struct EnvProvider {
    prefix: String,
    dotenv_path: Option<PathBuf>,
    dotenv_cache: Mutex<Option<HashMap<String, String>>>,
}

impl EnvProvider {
    /// Creates a new EnvProvider.
    pub fn new(prefix: impl Into<String>, dotenv_path: Option<PathBuf>) -> Self {
        Self {
            prefix: prefix.into(),
            dotenv_path,
            dotenv_cache: Default::default(),
        }
    }

    fn get_sync(&self, key: &Key) -> Result<Option<String>> {
        let env_key = format!("{}_{}", &self.prefix, key.as_ref().to_ascii_uppercase());
        match std::env::var(&env_key) {
            Err(std::env::VarError::NotPresent) => self.get_dotenv(&env_key),
            other => other
                .map(Some)
                .with_context(|| format!("failed to resolve env var {}", &env_key)),
        }
    }

    fn get_dotenv(&self, key: &str) -> Result<Option<String>> {
        if self.dotenv_path.is_none() {
            return Ok(None);
        }
        let mut maybe_cache = self
            .dotenv_cache
            .lock()
            .expect("dotenv_cache lock poisoned");
        let cache = match maybe_cache.as_mut() {
            Some(cache) => cache,
            None => maybe_cache.insert(self.load_dotenv()?),
        };
        Ok(cache.get(key).cloned())
    }

    fn load_dotenv(&self) -> Result<HashMap<String, String>> {
        let path = self.dotenv_path.as_deref().unwrap();
        Ok(dotenvy::from_path_iter(path)
            .into_iter()
            .flatten()
            .collect::<Result<HashMap<String, String>, _>>()?)
    }
}

#[async_trait]
impl Provider for EnvProvider {
    async fn get(&self, key: &Key) -> Result<Option<String>> {
        tokio::task::block_in_place(|| self.get_sync(key))
    }
}

#[cfg(test)]
mod test {
    use std::env::temp_dir;

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
            EnvProvider::new("TESTING_SPIN", None)
                .get_sync(&key1)
                .unwrap(),
            Some("val".to_string())
        );
    }

    #[test]
    fn provider_get_dotenv() {
        let dotenv_path = temp_dir().join("spin-env-provider-test");
        std::fs::write(&dotenv_path, b"TESTING_SPIN_ENV_KEY2=dotenv_val").unwrap();

        let key = Key::new("env_key2").unwrap();
        assert_eq!(
            EnvProvider::new("TESTING_SPIN", Some(dotenv_path))
                .get_sync(&key)
                .unwrap(),
            Some("dotenv_val".to_string())
        );
    }

    #[test]
    fn provider_get_missing() {
        let key = Key::new("please_do_not_ever_set_this_during_tests").unwrap();
        assert_eq!(
            EnvProvider::new("TESTING_SPIN", Default::default())
                .get_sync(&key)
                .unwrap(),
            None
        );
    }
}
