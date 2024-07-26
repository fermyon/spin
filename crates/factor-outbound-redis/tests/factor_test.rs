use anyhow::bail;
use spin_factor_outbound_networking::OutboundNetworkingFactor;
use spin_factor_outbound_redis::OutboundRedisFactor;
use spin_factor_variables::spin_cli::{StaticVariables, VariablesFactor};
use spin_factor_wasi::{DummyFilesMounter, WasiFactor};
use spin_factors::{anyhow, RuntimeFactors};
use spin_factors_test::{toml, TestEnvironment};
use spin_world::v2::redis::{Error, HostConnection};

#[derive(RuntimeFactors)]
struct TestFactors {
    wasi: WasiFactor,
    variables: VariablesFactor,
    networking: OutboundNetworkingFactor,
    redis: OutboundRedisFactor,
}

fn get_test_factors() -> TestFactors {
    TestFactors {
        wasi: WasiFactor::new(DummyFilesMounter),
        variables: VariablesFactor::default(),
        networking: OutboundNetworkingFactor,
        redis: OutboundRedisFactor,
    }
}

#[tokio::test]
async fn no_outbound_hosts_fails() -> anyhow::Result<()> {
    let mut factors = get_test_factors();

    factors.variables.add_provider_resolver(StaticVariables)?;

    let env = TestEnvironment {
        manifest: toml! {
            spin_manifest_version = 2
            application.name = "test-app"
            [[trigger.test]]

            [component.test-component]
            source = "does-not-exist.wasm"
        },
        ..Default::default()
    };
    let mut state = env.build_instance_state(factors, ()).await?;
    let connection = state
        .redis
        .open("redis://redis.test:8080".to_string())
        .await;

    let Err(err) = connection else {
        bail!("expected Error, got Ok");
    };

    assert!(matches!(err, Error::InvalidAddress));
    Ok(())
}
