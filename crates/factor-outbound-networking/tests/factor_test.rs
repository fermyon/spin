use spin_factor_outbound_networking::OutboundNetworkingFactor;
use spin_factor_variables::spin_cli::VariablesFactor;
use spin_factor_wasi::{DummyFilesMounter, WasiFactor};
use spin_factors::{anyhow, RuntimeFactors};
use spin_factors_test::{toml, TestEnvironment};
use wasmtime_wasi::{bindings::sockets::instance_network::Host, SocketAddrUse, WasiView};

#[derive(RuntimeFactors)]
struct TestFactors {
    wasi: WasiFactor,
    variables: VariablesFactor,
    networking: OutboundNetworkingFactor,
}

fn test_env() -> TestEnvironment {
    TestEnvironment::default_manifest_extend(toml! {
        [component.test-component]
        source = "does-not-exist.wasm"
        allowed_outbound_hosts = ["*://192.0.2.1:12345"]
    })
}

#[tokio::test]
async fn configures_wasi_socket_addr_check() -> anyhow::Result<()> {
    let factors = TestFactors {
        wasi: WasiFactor::new(DummyFilesMounter),
        variables: VariablesFactor::default(),
        networking: OutboundNetworkingFactor,
    };

    let env = test_env();
    let mut state = env.build_instance_state(factors).await?;
    let mut wasi = WasiFactor::get_wasi_impl(&mut state).unwrap();

    let network_resource = wasi.instance_network()?;
    let network = wasi.table().get(&network_resource)?;

    network
        .check_socket_addr(
            "192.0.2.1:12345".parse().unwrap(),
            SocketAddrUse::TcpConnect,
        )
        .await?;
    for not_allowed in ["192.0.2.1:25", "192.0.2.2:12345"] {
        assert_eq!(
            network
                .check_socket_addr(not_allowed.parse().unwrap(), SocketAddrUse::TcpConnect)
                .await
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::PermissionDenied
        );
    }
    Ok(())
}

#[tokio::test]
async fn wasi_factor_is_optional() -> anyhow::Result<()> {
    #[derive(RuntimeFactors)]
    struct WithoutWasi {
        variables: VariablesFactor,
        networking: OutboundNetworkingFactor,
    }
    TestEnvironment::default()
        .build_instance_state(WithoutWasi {
            variables: VariablesFactor::default(),
            networking: OutboundNetworkingFactor,
        })
        .await?;
    Ok(())
}
