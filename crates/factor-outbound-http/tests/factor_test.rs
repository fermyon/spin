use std::time::Duration;

use anyhow::bail;
use http::Request;
use spin_factor_outbound_http::OutboundHttpFactor;
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
async fn disallowed_host_fails() -> anyhow::Result<()> {
    let factors = TestFactors {
        variables: VariablesFactor::default(),
        networking: OutboundNetworkingFactor,
        http: OutboundHttpFactor,
    };
    let env = TestEnvironment::new(factors).extend_manifest(toml! {
        [component.test-component]
        source = "does-not-exist.wasm"
        allowed_outbound_hosts = ["http://allowed.test"]
    });
    let mut state = env.build_instance_state().await?;
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

fn test_request_config() -> OutgoingRequestConfig {
    OutgoingRequestConfig {
        use_tls: false,
        connect_timeout: Duration::from_secs(60),
        first_byte_timeout: Duration::from_secs(60),
        between_bytes_timeout: Duration::from_secs(60),
    }
}
