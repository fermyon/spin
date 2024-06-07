use std::collections::HashSet;

use anyhow::bail;
use serde::de::DeserializeOwned;

use crate::Factor;

pub const NO_RUNTIME_CONFIG: &str = "<no runtime config>";

/// FactorRuntimeConfig represents an application's runtime configuration.
///
/// Runtime configuration is partitioned, with each partition being the
/// responsibility of exactly one [`crate::Factor`]. If configuration needs
/// to be shared between Factors, one Factor can be selected as the owner
/// and the others will have a dependency relationship with that owner.
pub trait FactorRuntimeConfig: DeserializeOwned {
    const KEY: &'static str;
}

impl FactorRuntimeConfig for () {
    const KEY: &'static str = NO_RUNTIME_CONFIG;
}

pub trait RuntimeConfigSource {
    /// Returns an iterator of factor config keys available in this source.
    ///
    /// Should only include keys that have been positively provided. A runtime
    /// may treat unrecognized keys as a warning or error.
    fn config_keys(&self) -> impl IntoIterator<Item = &str>;

    /// Returns deserialized runtime config of the given type for the given
    /// factor config key.
    ///
    /// Returns Ok(None) if no configuration is available for the given key.
    /// Returns Err if configuration is available but deserialization fails.
    fn get_config<T: DeserializeOwned>(&self, key: &str) -> anyhow::Result<Option<T>>;
}

impl RuntimeConfigSource for () {
    fn get_config<T: DeserializeOwned>(
        &self,
        _factor_config_key: &str,
    ) -> anyhow::Result<Option<T>> {
        Ok(None)
    }

    fn config_keys(&self) -> impl IntoIterator<Item = &str> {
        std::iter::empty()
    }
}

pub struct RuntimeConfigTracker<S> {
    source: S,
    used_keys: HashSet<&'static str>,
    unused_keys: HashSet<String>,
}

impl<S: RuntimeConfigSource> RuntimeConfigTracker<S> {
    #[doc(hidden)]
    pub fn new(source: S) -> Self {
        let unused_keys = source
            .config_keys()
            .into_iter()
            .map(ToOwned::to_owned)
            .collect();
        Self {
            source,
            used_keys: Default::default(),
            unused_keys,
        }
    }

    #[doc(hidden)]
    pub fn validate_all_keys_used(self) -> anyhow::Result<()> {
        if !self.unused_keys.is_empty() {
            bail!(
                "unused runtime config key(s): {keys}",
                keys = self
                    .unused_keys
                    .iter()
                    .map(|key| format!("{key:?}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        Ok(())
    }

    pub fn get_config<F: Factor>(&mut self) -> anyhow::Result<Option<F::RuntimeConfig>> {
        let key = F::RuntimeConfig::KEY;
        if key == NO_RUNTIME_CONFIG {
            return Ok(None);
        }
        if !self.used_keys.insert(key) {
            bail!("already used runtime config key {key:?}");
        }
        self.unused_keys.remove(key);
        self.source.get_config::<F::RuntimeConfig>(key)
    }
}
