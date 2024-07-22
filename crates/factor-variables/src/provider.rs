use serde::de::DeserializeOwned;
use spin_expressions::Provider;
use spin_factors::anyhow;

/// A trait for converting a runtime configuration into a variables provider.
pub trait MakeVariablesProvider: 'static {
    /// Serialized configuration for the provider.
    type RuntimeConfig: DeserializeOwned;

    /// Create a variables provider from the given runtime configuration.
    ///
    /// Returns `Ok(None)` if the provider is not applicable to the given configuration.
    fn make_provider(
        &self,
        runtime_config: &Self::RuntimeConfig,
    ) -> anyhow::Result<Option<Box<dyn Provider>>>;
}
