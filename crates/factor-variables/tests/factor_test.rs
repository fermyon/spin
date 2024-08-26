use spin_factor_variables::{spin_cli, VariablesFactor};
use spin_factors::{
    anyhow, Factor, FactorRuntimeConfigSource, RuntimeConfigSourceFinalizer, RuntimeFactors,
};
use spin_factors_test::{toml, TestEnvironment};
use spin_world::v2::variables::Host;

#[derive(RuntimeFactors)]
struct TestFactors {
    variables: VariablesFactor,
}

#[tokio::test(flavor = "multi_thread")]
async fn static_provider_works() -> anyhow::Result<()> {
    let factors = TestFactors {
        variables: VariablesFactor::default(),
    };
    let env = TestEnvironment::new(factors)
        .extend_manifest(toml! {
            [variables]
            foo = { required = true }

            [component.test-component]
            source = "does-not-exist.wasm"
            variables = { baz = "<{{ foo }}>" }
        })
        .runtime_config(TomlConfig::new(toml! {
            [[variable_provider]]
            type = "static"
            values = { foo = "bar" }
        }))?;

    let mut state = env.build_instance_state().await?;
    let val = state.variables.get("baz".into()).await?;
    assert_eq!(val, "<bar>");
    Ok(())
}

struct TomlConfig {
    table: toml::Table,
}

impl TomlConfig {
    fn new(table: toml::Table) -> Self {
        Self { table }
    }
}

impl TryFrom<TomlConfig> for TestFactorsRuntimeConfig {
    type Error = anyhow::Error;

    fn try_from(value: TomlConfig) -> Result<Self, Self::Error> {
        Self::from_source(value)
    }
}

impl FactorRuntimeConfigSource<VariablesFactor> for TomlConfig {
    fn get_runtime_config(
        &mut self,
    ) -> anyhow::Result<Option<<VariablesFactor as Factor>::RuntimeConfig>> {
        spin_cli::runtime_config_from_toml(&self.table).map(Some)
    }
}

impl RuntimeConfigSourceFinalizer for TomlConfig {
    fn finalize(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}
