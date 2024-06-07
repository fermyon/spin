use spin_app::App;
use spin_factor_outbound_networking::OutboundNetworkingFactor;
use spin_factor_variables::{StaticVariables, VariablesFactor};
use spin_factor_wasi::{preview1::WasiPreview1Factor, DummyFilesMounter, WasiFactor};
use spin_factors::{FactorRuntimeConfig, RuntimeConfigSource, RuntimeFactors};

#[derive(RuntimeFactors)]
struct Factors {
    wasi: WasiFactor,
    wasip1: WasiPreview1Factor,
    variables: VariablesFactor,
    outbound_networking_factor: OutboundNetworkingFactor,
}

#[tokio::test(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    let mut factors = Factors {
        wasi: WasiFactor::new(DummyFilesMounter),
        wasip1: WasiPreview1Factor,
        variables: VariablesFactor::default(),
        outbound_networking_factor: OutboundNetworkingFactor,
        // outbound_http_factor: OutboundHttpFactor,
    };
    factors.variables.add_provider_type(StaticVariables)?;

    let locked = serde_json::from_value(serde_json::json!({
        "spin_lock_version": 1,
        "variables": {
            "foo": {}
        },
        "triggers": [],
        "components": [{
            "id": "test",
            "metadata": {
                "allowed_outbound_hosts": ["http://{{ foo }}"]
            },
            "source": {
                "content_type": "application/wasm",
                "content": {"inline": "KGNvbXBvbmVudCk="}
            },
            "config": {
                "test_var": "{{foo}}"
            }
        }]
    }))?;
    let app = App::inert(locked);

    let engine = wasmtime::Engine::new(wasmtime::Config::new().async_support(true))?;
    let mut linker = wasmtime::component::Linker::new(&engine);
    let mut module_linker = wasmtime::Linker::new(&engine);

    factors
        .init(Some(&mut linker), Some(&mut module_linker))
        .unwrap();

    let configured_app = factors.configure_app(app, TestSource)?;
    let data = factors.build_store_data(&configured_app, "test")?;

    assert_eq!(
        data.variables
            .resolver()
            .resolve("test", "test_var".try_into().unwrap())
            .await
            .unwrap(),
        "bar"
    );

    let mut store = wasmtime::Store::new(&engine, data);

    let component = wasmtime::component::Component::new(&engine, b"(component)")?;
    let _instance = linker.instantiate_async(&mut store, &component).await?;

    let module = wasmtime::Module::new(&engine, b"(module)")?;
    let _module_instance = module_linker.instantiate_async(&mut store, &module).await?;

    Ok(())
}

struct TestSource;

impl RuntimeConfigSource for TestSource {
    fn config_keys(&self) -> impl IntoIterator<Item = &str> {
        [spin_factor_variables::RuntimeConfig::KEY]
    }

    fn get_config<T: serde::de::DeserializeOwned>(&self, key: &str) -> anyhow::Result<Option<T>> {
        let Some(table) = toml::toml! {
            [[variable_provider]]
            type = "static"
            [variable_provider.values]
            foo = "bar"
        }
        .remove(key) else {
            return Ok(None);
        };
        let config = table.try_into()?;
        Ok(Some(config))
    }
}
