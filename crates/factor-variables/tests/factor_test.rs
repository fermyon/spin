use spin_factor_variables::spin_cli::{RuntimeConfig, StaticVariables};
use spin_factor_variables::VariablesFactor;
use spin_factors::{anyhow, RuntimeFactors};
use spin_factors_test::{toml, TestEnvironment};

#[derive(RuntimeFactors)]
struct TestFactors {
    variables: VariablesFactor<RuntimeConfig>,
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
    factors.variables.add_provider_type(StaticVariables)?;

    let env = test_env();
    let state = env.build_instance_state(factors).await?;
    let val = state
        .variables
        .resolver()
        .resolve("test-component", "baz".try_into().unwrap())
        .await?;
    assert_eq!(val, "<bar>");
    Ok(())
}
