use std::{collections::HashMap, sync::Arc};

use serde::Deserialize;
use spin_expressions::{async_trait::async_trait, Key, Provider};
use spin_factors::anyhow;

use crate::MakeVariablesProvider;

/// Creator of a static variables provider.
pub struct StaticVariables;

impl MakeVariablesProvider for StaticVariables {
    const RUNTIME_CONFIG_TYPE: &'static str = "static";

    type RuntimeConfig = StaticVariablesProvider;
    type Provider = StaticVariablesProvider;

    fn make_provider(&self, runtime_config: Self::RuntimeConfig) -> anyhow::Result<Self::Provider> {
        Ok(runtime_config)
    }
}

/// A variables provider that reads variables from an static map.
#[derive(Debug, Deserialize)]
pub struct StaticVariablesProvider {
    values: Arc<HashMap<String, String>>,
}

#[async_trait]
impl Provider for StaticVariablesProvider {
    async fn get(&self, key: &Key) -> anyhow::Result<Option<String>> {
        Ok(self.values.get(key.as_str()).cloned())
    }
}
