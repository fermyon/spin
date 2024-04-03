use std::{collections::HashMap, path::PathBuf, sync::Mutex};

use anyhow::{Context, Result};
use async_trait::async_trait;

use spin_expressions::{Key, Provider};
use tracing::{instrument, Level};

const DEFAULT_ENV_PREFIX: &str = "SPIN_VARIABLE";
const LEGACY_ENV_PREFIX: &str = "SPIN_CONFIG";

/// A config Provider that uses environment variables.
#[derive(Debug)]
pub struct EnvProvider {
    prefix: Option<String>,
    dotenv_path: Option<PathBuf>,
    dotenv_cache: Mutex<Option<HashMap<String, String>>>,
}

impl EnvProvider {
    /// Creates a new EnvProvider.
    pub fn new(prefix: Option<impl Into<String>>, dotenv_path: Option<PathBuf>) -> Self {
        Self {
            prefix: prefix.map(Into::into),
            dotenv_path,
            dotenv_cache: Default::default(),
        }
    }

    fn query_env(&self, env_key: &str) -> Result<Option<String>> {
        match std::env::var(env_key) {
            Err(std::env::VarError::NotPresent) => self.get_dotenv(env_key),
            other => other
                .map(Some)
                .with_context(|| format!("failed to resolve env var {env_key}")),
        }
    }

    fn get_sync(&self, key: &Key) -> Result<Option<String>> {
        let prefix = self
            .prefix
            .clone()
            .unwrap_or(DEFAULT_ENV_PREFIX.to_string());
        let use_fallback = self.prefix.is_none();

        let upper_key = key.as_ref().to_ascii_uppercase();
        let env_key = format!("{prefix}_{upper_key}");

        match self.query_env(&env_key)? {
            None if use_fallback => {
                let old_key = format!("{LEGACY_ENV_PREFIX}_{upper_key}");
                let result = self.query_env(&old_key);
                if let Ok(Some(_)) = &result {
                    eprintln!("Warning: variable '{key}': {env_key} was not set, so used {old_key}. The {LEGACY_ENV_PREFIX} prefix is deprecated; please switch to the {DEFAULT_ENV_PREFIX} prefix.", key = key.as_ref());
                }
                result
            }
            other => Ok(other),
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
    #[instrument(name = "spin_variables.get_from_env", skip(self), err(level = Level::INFO))]
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
            EnvProvider::new(Some("TESTING_SPIN"), None)
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
            EnvProvider::new(Some("TESTING_SPIN"), Some(dotenv_path))
                .get_sync(&key)
                .unwrap(),
            Some("dotenv_val".to_string())
        );
    }

    #[test]
    fn provider_get_missing() {
        let key = Key::new("please_do_not_ever_set_this_during_tests").unwrap();
        assert_eq!(
            EnvProvider::new(Some("TESTING_SPIN"), Default::default())
                .get_sync(&key)
                .unwrap(),
            None
        );
    }
}
