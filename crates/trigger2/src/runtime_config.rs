use spin_factor_wasi::WasiFactor;
use spin_factors::{
    runtime_config::toml::TomlKeyTracker, FactorRuntimeConfigSource, RuntimeConfigSourceFinalizer,
};

use crate::factors::TriggerFactorsRuntimeConfig;

/// A runtime configuration source for the [`TriggerFactors`][crate::TriggerFactors].
pub struct RuntimeConfigSource<'a> {
    table: TomlKeyTracker<'a>,
}

impl<'a> RuntimeConfigSource<'a> {
    pub fn new(table: &'a toml::Table) -> Self {
        Self {
            table: TomlKeyTracker::new(table),
        }
    }
}

impl RuntimeConfigSourceFinalizer for RuntimeConfigSource<'_> {
    fn finalize(&mut self) -> anyhow::Result<()> {
        Ok(self.table.validate_all_keys_used()?)
    }
}

impl FactorRuntimeConfigSource<WasiFactor> for RuntimeConfigSource<'_> {
    fn get_runtime_config(&mut self) -> anyhow::Result<Option<()>> {
        Ok(None)
    }
}

impl TryFrom<RuntimeConfigSource<'_>> for TriggerFactorsRuntimeConfig {
    type Error = anyhow::Error;

    fn try_from(value: RuntimeConfigSource<'_>) -> Result<Self, Self::Error> {
        Self::from_source(value)
    }
}
