use anyhow::bail;
use spin_factor_outbound_networking::OutboundNetworkingFactor;
use spin_factor_outbound_pg::OutboundPgFactor;
use spin_factor_variables::{StaticVariables, VariablesFactor};
use spin_factor_wasi::{DummyFilesMounter, WasiFactor};
use spin_factors::{anyhow, RuntimeFactors};
use spin_factors_test::{toml, TestEnvironment};
use spin_world::v2::postgres::HostConnection;
use spin_world::v2::rdbms_types::Error as PgError;

#[derive(RuntimeFactors)]
struct TestFactors {
    wasi: WasiFactor,
    variables: VariablesFactor,
    networking: OutboundNetworkingFactor,
    pg: OutboundPgFactor,
}

fn test_env() -> TestEnvironment {
    TestEnvironment::default_manifest_extend(toml! {
        [component.test-component]
        source = "does-not-exist.wasm"
    })
}

#[tokio::test]
async fn disallowed_host_fails() -> anyhow::Result<()> {
    let mut factors = TestFactors {
        wasi: WasiFactor::new(DummyFilesMounter),
        variables: VariablesFactor::default(),
        networking: OutboundNetworkingFactor,
        pg: OutboundPgFactor,
    };
    factors.variables.add_provider_type(StaticVariables)?;

    let env = test_env();
    let mut state = env.build_instance_state(factors).await?;

    let res = state
        .pg
        .open("postgres://postgres.test:5432/test".to_string())
        .await;
    let Err(err) = res else {
        bail!("expected Err, got Ok");
    };
    println!("err: {:?}", err);
    assert!(matches!(err, PgError::ConnectionFailed(_)));

    Ok(())
}
