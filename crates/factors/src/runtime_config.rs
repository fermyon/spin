pub mod toml;

use crate::Factor;

/// The source of runtime configuration for a particular [`Factor`].
pub trait FactorRuntimeConfigSource<F: Factor> {
    /// Get the runtime configuration for the factor.
    fn get_runtime_config(&mut self) -> anyhow::Result<Option<F::RuntimeConfig>>;
}

impl<F: Factor> FactorRuntimeConfigSource<F> for () {
    fn get_runtime_config(&mut self) -> anyhow::Result<Option<<F as Factor>::RuntimeConfig>> {
        Ok(None)
    }
}

/// Run some finalization logic on a [`FactorRuntimeConfigSource`].
pub trait RuntimeConfigSourceFinalizer {
    /// Finalize the runtime config source.
    fn finalize(&mut self) -> anyhow::Result<()>;
}

impl RuntimeConfigSourceFinalizer for () {
    fn finalize(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}
