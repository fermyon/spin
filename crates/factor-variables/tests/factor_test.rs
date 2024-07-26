use spin_factor_variables::spin_cli::{
    EnvVariables, StaticVariables, VariableProviderConfiguration,
};
use spin_factor_variables::VariablesFactor;
use spin_factors::{
    anyhow, Factor, FactorRuntimeConfigSource, RuntimeConfigSourceFinalizer, RuntimeFactors,
};
use spin_factors_test::{toml, TestEnvironment};
use spin_world::v2::variables::Host;

#[derive(RuntimeFactors)]
struct TestFactors {
    variables: VariablesFactor<VariableProviderConfiguration>,
}

fn test_env() -> TestEnvironment {
    TestEnvironment::default_manifest_extend(toml! {
        [variables]
        foo = { required = true }

        [component.test-component]
        source = "does-not-exist.wasm"
        variables = { baz = "<{{ foo }}>" }
    })
}

#[tokio::test]
async fn static_provider_works() -> anyhow::Result<()> {
    let runtime_config = toml! {
        [[variable_provider]]
        type = "static"
        values = { foo = "bar" }
    };
    let mut factors = TestFactors {
        variables: VariablesFactor::default(),
    };
    factors.variables.add_provider_resolver(StaticVariables)?;
    // The env provider will be ignored since there's no configuration for it.
    factors.variables.add_provider_resolver(EnvVariables)?;

    let env = test_env();
    let mut state = env
        .build_instance_state(factors, TomlConfig(runtime_config))
        .await?;
    let val = state.variables.get("baz".try_into().unwrap()).await?;
    assert_eq!(val, "<bar>");
    Ok(())
}

struct TomlConfig(toml::Table);

impl TryFrom<TomlConfig> for TestFactorsRuntimeConfig {
    type Error = anyhow::Error;

    fn try_from(value: TomlConfig) -> Result<Self, Self::Error> {
        Self::from_source(value)
    }
}

impl FactorRuntimeConfigSource<VariablesFactor<VariableProviderConfiguration>> for TomlConfig {
    fn get_runtime_config(
        &mut self,
    ) -> anyhow::Result<
        Option<<VariablesFactor<VariableProviderConfiguration> as Factor>::RuntimeConfig>,
    > {
        let Some(table) = self.0.get("variable_provider") else {
            return Ok(None);
        };
        Ok(Some(table.clone().try_into()?))
    }
}

impl RuntimeConfigSourceFinalizer for TomlConfig {
    fn finalize(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}
