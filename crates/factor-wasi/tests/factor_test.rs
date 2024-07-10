use spin_factor_wasi::{DummyFilesMounter, WasiFactor};
use spin_factors::{anyhow, RuntimeFactors};
use spin_factors_test::{toml, TestEnvironment};
use wasmtime_wasi::{bindings::cli::environment::Host, WasiImpl};

#[derive(RuntimeFactors)]
struct TestFactors {
    wasi: WasiFactor,
}

#[tokio::test]
async fn environment_works() -> anyhow::Result<()> {
    let factors = TestFactors {
        wasi: WasiFactor::new(DummyFilesMounter),
    };

    let mut env = TestEnvironment {
        manifest: toml! {
            spin_manifest_version = 2
            application.name = "test-app"
            [[trigger.test]]

            [component.test-component]
            source = "does-not-exist.wasm"
            environment = { FOO = "bar" }
        },
        ..Default::default()
    };
    let mut state = env.build_instance_state(factors).await?;
    let mut wasi = WasiImpl(&mut state.wasi);
    let val = wasi
        .get_environment()?
        .into_iter()
        .find_map(|(key, val)| (key == "FOO").then_some(val));
    assert_eq!(val.as_deref(), Some("bar"));
    Ok(())
}
