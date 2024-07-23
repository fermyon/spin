use spin_factor_variables::spin_cli::{
    EnvVariables, StaticVariables, VariableProviderConfiguration,
};
use spin_factor_variables::VariablesFactor;
use spin_factors::{anyhow, RuntimeFactors};
use spin_factors_test::{toml, TestEnvironment};
use spin_world::v2::variables::Host;

#[derive(RuntimeFactors)]
struct TestFactors {
    variables: VariablesFactor<VariableProviderConfiguration>,
}

fn test_env() -> TestEnvironment {
    let mut env = TestEnvironment::default_manifest_extend(toml! {
        [variables]
        foo = { required = true }

        [component.test-component]
        source = "does-not-exist.wasm"
        variables = { baz = "<{{ foo }}>" }
    });
    env.runtime_config = toml! {
        [[variable_provider]]
        type = "static"
        values = { foo = "bar" }
    };
    env
}

#[tokio::test]
async fn static_provider_works() -> anyhow::Result<()> {
    let mut factors = TestFactors {
        variables: VariablesFactor::default(),
    };
    factors.variables.add_provider_resolver(StaticVariables)?;
    // The env provider will be ignored since there's no configuration for it.
    factors.variables.add_provider_resolver(EnvVariables)?;

    let env = test_env();
    let mut state = env.build_instance_state(factors).await?;
    let val = state.variables.get("baz".try_into().unwrap()).await?;
    assert_eq!(val, "<bar>");
    Ok(())
}
