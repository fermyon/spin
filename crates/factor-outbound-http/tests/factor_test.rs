use std::time::Duration;

use anyhow::bail;
use http::{Request, Uri};
use spin_factor_outbound_http::{OutboundHttpFactor, SelfRequestOrigin};
use spin_factor_outbound_networking::OutboundNetworkingFactor;
use spin_factor_variables::VariablesFactor;
use spin_factors::{anyhow, RuntimeFactors};
use spin_factors_test::{toml, TestEnvironment};
use wasmtime_wasi::Subscribe;
use wasmtime_wasi_http::{
    bindings::http::types::ErrorCode, types::OutgoingRequestConfig, WasiHttpView,
};

#[derive(RuntimeFactors)]
struct TestFactors {
    variables: VariablesFactor,
    networking: OutboundNetworkingFactor,
    http: OutboundHttpFactor,
}

#[tokio::test]
async fn allowed_host_is_allowed() -> anyhow::Result<()> {
    let mut state = test_instance_state("https://*").await?;
    let mut wasi_http = OutboundHttpFactor::get_wasi_http_impl(&mut state).unwrap();

    // [100::] is an IPv6 "black hole", which should always fail
    let req = Request::get("https://[100::1]:443").body(Default::default())?;
    let mut future_resp = wasi_http.send_request(req, test_request_config())?;
    future_resp.ready().await;

    // We don't want to make an actual network request, so treat "connection refused" as success
    match future_resp.unwrap_ready().unwrap() {
        Ok(_) => bail!("expected Err, got Ok"),
        Err(err) => assert!(matches!(err, ErrorCode::ConnectionRefused), "{err:?}"),
    };
    Ok(())
}

#[tokio::test]
async fn self_request_smoke_test() -> anyhow::Result<()> {
    let mut state = test_instance_state("http://self").await?;
    let mut wasi_http = OutboundHttpFactor::get_wasi_http_impl(&mut state).unwrap();

    let mut req = Request::get("/self-request").body(Default::default())?;
    let origin = Uri::from_static("http://[100::1]");
    req.extensions_mut()
        .insert(SelfRequestOrigin::from_uri(&origin).unwrap());
    let mut future_resp = wasi_http.send_request(req, test_request_config())?;
    future_resp.ready().await;

    // We don't want to make an actual network request, so treat "connection refused" as success
    match future_resp.unwrap_ready().unwrap() {
        Ok(_) => bail!("expected Err, got Ok"),
        Err(err) => assert!(matches!(err, ErrorCode::ConnectionRefused), "{err:?}"),
    };
    Ok(())
}

#[tokio::test]
async fn disallowed_host_fails() -> anyhow::Result<()> {
    let mut state = test_instance_state("https://allowed.test").await?;
    let mut wasi_http = OutboundHttpFactor::get_wasi_http_impl(&mut state).unwrap();

    let req = Request::get("https://denied.test").body(Default::default())?;
    let mut future_resp = wasi_http.send_request(req, test_request_config())?;
    future_resp.ready().await;
    match future_resp.unwrap_ready() {
        Ok(_) => bail!("expected Err, got Ok"),
        Err(err) => assert!(matches!(err.downcast()?, ErrorCode::HttpRequestDenied)),
    };
    Ok(())
}

async fn test_instance_state(
    allowed_outbound_hosts: &str,
) -> anyhow::Result<TestFactorsInstanceState> {
    let factors = TestFactors {
        variables: VariablesFactor::default(),
        networking: OutboundNetworkingFactor,
        http: OutboundHttpFactor,
    };
    let env = TestEnvironment::new(factors).extend_manifest(toml! {
        [component.test-component]
        source = "does-not-exist.wasm"
        allowed_outbound_hosts = [allowed_outbound_hosts]
    });
    env.build_instance_state().await
}

fn test_request_config() -> OutgoingRequestConfig {
    OutgoingRequestConfig {
        use_tls: false,
        connect_timeout: Duration::from_secs(60),
        first_byte_timeout: Duration::from_secs(60),
        between_bytes_timeout: Duration::from_secs(60),
    }
}
