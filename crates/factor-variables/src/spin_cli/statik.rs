use std::{collections::HashMap, sync::Arc};

use serde::Deserialize;
use spin_expressions::{async_trait::async_trait, Key, Provider};
use spin_factors::anyhow;

use crate::ProviderResolver;

use super::VariableProviderConfiguration;

/// Creator of a static variables provider.
pub struct StaticVariables;

impl ProviderResolver for StaticVariables {
    type RuntimeConfig = VariableProviderConfiguration;

    fn resolve_provider(
        &self,
        runtime_config: &Self::RuntimeConfig,
    ) -> anyhow::Result<Option<Box<dyn Provider>>> {
        let VariableProviderConfiguration::Static(config) = runtime_config else {
            return Ok(None);
        };
        Ok(Some(Box::new(config.clone()) as _))
    }
}

/// A variables provider that reads variables from an static map.
#[derive(Debug, Deserialize, Clone)]
pub struct StaticVariablesProvider {
    values: Arc<HashMap<String, String>>,
}

#[async_trait]
impl Provider for StaticVariablesProvider {
    async fn get(&self, key: &Key) -> anyhow::Result<Option<String>> {
        Ok(self.values.get(key.as_str()).cloned())
    }
}
