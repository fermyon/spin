use std::collections::HashSet;

use anyhow::bail;
use serde::de::DeserializeOwned;

/// RuntimeConfig represents an application's runtime configuration.
///
/// Runtime configuration is partitioned, with each partition being the
/// responsibility of exactly one [`crate::Factor`]. If configuration needs to
/// be shared between Factors, one Factor can be selected as the owner and the
/// others will have a dependency relationship with that owner.
pub trait RuntimeConfig {
    /// Returns deserialized runtime config of the given type for the given
    /// factor config key.
    ///
    /// Returns Ok(None) if no configuration is available for the given key.
    /// Returns Err if configuration is available but deserialization fails,
    /// or if the given config key has already been retrieved.
    fn get_config<T: DeserializeOwned>(
        &mut self,
        factor_config_key: &str,
    ) -> anyhow::Result<Option<T>>;
}

pub struct RuntimeConfigTracker<Source> {
    source: Source,
    used_keys: HashSet<String>,
    unused_keys: HashSet<String>,
}

impl<Source: RuntimeConfigSource> RuntimeConfigTracker<Source> {
    #[doc(hidden)]
    pub fn new(source: Source) -> Self {
        let unused_keys = source.factor_config_keys().map(ToOwned::to_owned).collect();
        Self {
            source,
            used_keys: Default::default(),
            unused_keys,
        }
    }

    #[doc(hidden)]
    pub fn validate_all_keys_used(self) -> Result<(), impl IntoIterator<Item = String>> {
        if self.unused_keys.is_empty() {
            Ok(())
        } else {
            Err(self.unused_keys)
        }
    }
}

impl<Source: RuntimeConfigSource> RuntimeConfig for RuntimeConfigTracker<Source> {
    fn get_config<T: DeserializeOwned>(
        &mut self,
        factor_config_key: &str,
    ) -> anyhow::Result<Option<T>> {
        if !self.used_keys.insert(factor_config_key.to_owned()) {
            bail!("already got runtime config key {factor_config_key:?}");
        }
        self.unused_keys.remove(factor_config_key);
        self.source.get_config::<T>(factor_config_key)
    }
}

pub trait RuntimeConfigSource {
    /// Returns deserialized runtime config of the given type for the given
    /// factor config key.
    ///
    /// Returns Ok(None) if no configuration is available for the given key.
    /// Returns Err if configuration is available but deserialization fails.
    fn get_config<Config: DeserializeOwned>(
        &self,
        factor_config_key: &str,
    ) -> anyhow::Result<Option<Config>>;

    /// Returns an iterator of factor config keys available in this source.
    ///
    /// Should only include keys that have been positively provided. A runtime
    /// may treat unrecognized keys as a warning or error.
    fn factor_config_keys(&self) -> impl Iterator<Item = &str>;
}

impl RuntimeConfigSource for () {
    fn get_config<T: DeserializeOwned>(
        &self,
        _factor_config_key: &str,
    ) -> anyhow::Result<Option<T>> {
        Ok(None)
    }

    fn factor_config_keys(&self) -> impl Iterator<Item = &str> {
        std::iter::empty()
    }
}
