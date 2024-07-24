use crate::Factor;

/// The source of runtime configuration for a Factor.
pub trait RuntimeConfigSource {
    /// Returns deserialized runtime config of the given type for the given
    /// factor config key.
    ///
    /// Returns Ok(None) if no configuration is available for the given key.
    /// Returns Err if configuration is available but deserialization fails.
    fn get_factor_config<F: Factor>(&self) -> anyhow::Result<Option<F::RuntimeConfig>>;
}

impl RuntimeConfigSource for () {
    fn get_factor_config<F: Factor>(&self) -> anyhow::Result<Option<F::RuntimeConfig>> {
        Ok(None)
    }
}
