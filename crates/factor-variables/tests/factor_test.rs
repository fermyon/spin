use spin_factor_variables::{StaticVariables, VariablesFactor};
use spin_factors::{anyhow, RuntimeFactors};
use spin_factors_test::{toml, TestEnvironment};

#[derive(RuntimeFactors)]
struct TestFactors {
    variables: VariablesFactor,
}

#[tokio::test]
async fn static_provider_works() -> anyhow::Result<()> {
    let mut factors = TestFactors {
        variables: VariablesFactor::default(),
    };
    factors.variables.add_provider_type(StaticVariables)?;

    let mut env = TestEnvironment {
        manifest: toml! {
            spin_manifest_version = 2
            application.name = "test-app"
            [[trigger.test]]

            [variables]
            foo = { required = true }

            [component.test-component]
            source = "does-not-exist.wasm"
            variables = { baz = "<{{ foo }}>" }
        },
        runtime_config: toml! {
            [[variable_provider]]
            type = "static"
            values = { foo = "bar" }
        },
    };
    let state = env.build_instance_state(factors).await?;
    let val = state
        .variables
        .resolver()
        .resolve("test-component", "baz".try_into().unwrap())
        .await?;
    assert_eq!(val, "<bar>");
    Ok(())
}
