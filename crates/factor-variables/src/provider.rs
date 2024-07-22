mod env;
mod statik;

pub use env::EnvVariables;
pub use statik::StaticVariables;

use serde::de::DeserializeOwned;
use spin_expressions::Provider;
use spin_factors::anyhow;

pub trait MakeVariablesProvider: 'static {
    const RUNTIME_CONFIG_TYPE: &'static str;

    type RuntimeConfig: DeserializeOwned;
    type Provider: Provider;

    fn make_provider(&self, runtime_config: Self::RuntimeConfig) -> anyhow::Result<Self::Provider>;
}

pub(crate) type ProviderFromToml = Box<dyn Fn(toml::Table) -> anyhow::Result<Box<dyn Provider>>>;

pub(crate) fn provider_from_toml_fn<T: MakeVariablesProvider>(
    provider_type: T,
) -> ProviderFromToml {
    Box::new(move |table| {
        let runtime_config: T::RuntimeConfig = table.try_into()?;
        let provider = provider_type.make_provider(runtime_config)?;
        Ok(Box::new(provider))
    })
}
