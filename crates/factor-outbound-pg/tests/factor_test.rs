use anyhow::{bail, Result};
use spin_factor_outbound_networking::OutboundNetworkingFactor;
use spin_factor_outbound_pg::client::Client;
use spin_factor_outbound_pg::OutboundPgFactor;
use spin_factor_variables::spin_cli::{StaticVariables, VariablesFactor};
use spin_factors::{anyhow, RuntimeFactors};
use spin_factors_test::{toml, TestEnvironment};
use spin_world::async_trait;
use spin_world::v2::postgres::HostConnection;
use spin_world::v2::postgres::{self as v2};
use spin_world::v2::rdbms_types::Error as PgError;
use spin_world::v2::rdbms_types::{ParameterValue, RowSet};

#[derive(RuntimeFactors)]
struct TestFactors {
    variables: VariablesFactor,
    networking: OutboundNetworkingFactor,
    pg: OutboundPgFactor<MockClient>,
}

fn factors() -> Result<TestFactors> {
    let mut f = TestFactors {
        variables: VariablesFactor::default(),
        networking: OutboundNetworkingFactor,
        pg: OutboundPgFactor::<MockClient>::new(),
    };
    f.variables.add_provider_type(StaticVariables)?;
    Ok(f)
}

fn test_env() -> TestEnvironment {
    TestEnvironment::default_manifest_extend(toml! {
        [component.test-component]
        source = "does-not-exist.wasm"
        allowed_outbound_hosts = ["postgres://*:*"]
    })
}

#[tokio::test]
async fn disallowed_host_fails() -> anyhow::Result<()> {
    let factors = factors()?;
    let env = TestEnvironment::default_manifest_extend(toml! {
        [component.test-component]
        source = "does-not-exist.wasm"
    });
    let mut state = env.build_instance_state(factors).await?;

    let res = state
        .pg
        .open("postgres://postgres.test:5432/test".to_string())
        .await;
    let Err(err) = res else {
        bail!("expected Err, got Ok");
    };
    assert!(matches!(err, PgError::ConnectionFailed(_)));

    Ok(())
}

#[tokio::test]
async fn allowed_host_succeeds() -> anyhow::Result<()> {
    let factors = factors()?;
    let env = test_env();
    let mut state = env.build_instance_state(factors).await?;

    let res = state
        .pg
        .open("postgres://localhost:5432/test".to_string())
        .await;
    let Ok(_) = res else {
        bail!("expected Ok, got Err");
    };

    Ok(())
}

#[tokio::test]
async fn exercise_execute() -> anyhow::Result<()> {
    let factors = factors()?;
    let env = test_env();
    let mut state = env.build_instance_state(factors).await?;

    let connection = state
        .pg
        .open("postgres://localhost:5432/test".to_string())
        .await?;

    state
        .pg
        .execute(connection, "SELECT * FROM test".to_string(), vec![])
        .await?;

    Ok(())
}

#[tokio::test]
async fn exercise_query() -> anyhow::Result<()> {
    let factors = factors()?;
    let env = test_env();
    let mut state = env.build_instance_state(factors).await?;

    let connection = state
        .pg
        .open("postgres://localhost:5432/test".to_string())
        .await?;

    state
        .pg
        .query(connection, "SELECT * FROM test".to_string(), vec![])
        .await?;

    Ok(())
}

// TODO: We can expand this mock to track calls and simulate return values
pub struct MockClient {}

#[async_trait]
impl Client for MockClient {
    async fn build_client(_address: &str) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(MockClient {})
    }

    async fn execute(
        &self,
        _statement: String,
        _params: Vec<ParameterValue>,
    ) -> Result<u64, v2::Error> {
        Ok(0)
    }

    async fn query(
        &self,
        _statement: String,
        _params: Vec<ParameterValue>,
    ) -> Result<RowSet, v2::Error> {
        Ok(RowSet {
            columns: vec![],
            rows: vec![],
        })
    }
}
