use std::{collections::HashMap, sync::Arc};

use serde::{de::DeserializeOwned, Deserialize};
use spin_expressions::{async_trait::async_trait, Key, Provider};
use spin_factors::anyhow;

pub trait MakeVariablesProvider: 'static {
    const TYPE: &'static str;

    type RuntimeConfig: DeserializeOwned;
    type Provider: Provider;

    fn make_provider(&self, runtime_config: Self::RuntimeConfig) -> anyhow::Result<Self::Provider>;
}

pub(crate) type ProviderMaker = Box<dyn Fn(toml::Table) -> anyhow::Result<Box<dyn Provider>>>;

pub(crate) fn provider_maker<T: MakeVariablesProvider>(provider_type: T) -> ProviderMaker {
    Box::new(move |table| {
        let runtime_config: T::RuntimeConfig = table.try_into()?;
        let provider = provider_type.make_provider(runtime_config)?;
        Ok(Box::new(provider))
    })
}

pub struct StaticVariables;

impl MakeVariablesProvider for StaticVariables {
    const TYPE: &'static str = "static";

    type RuntimeConfig = StaticVariablesProvider;
    type Provider = StaticVariablesProvider;

    fn make_provider(&self, runtime_config: Self::RuntimeConfig) -> anyhow::Result<Self::Provider> {
        Ok(runtime_config)
    }
}

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
