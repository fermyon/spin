use std::path::PathBuf;

use anyhow::{bail, Context};
use http_body_util::BodyExt;
use serde::Deserialize;
use spin_app::App;
use spin_factor_key_value::{KeyValueFactor, MakeKeyValueStore};
use spin_factor_outbound_http::OutboundHttpFactor;
use spin_factor_outbound_networking::OutboundNetworkingFactor;
use spin_factor_variables::{StaticVariables, VariablesFactor};
use spin_factor_wasi::{DummyFilesMounter, WasiFactor};
use spin_factors::{FactorRuntimeConfig, RuntimeConfigSource, RuntimeFactors};
use spin_key_value_sqlite::{DatabaseLocation, KeyValueSqlite};
use wasmtime_wasi_http::WasiHttpView;

#[derive(RuntimeFactors)]
struct Factors {
    wasi: WasiFactor,
    variables: VariablesFactor,
    outbound_networking: OutboundNetworkingFactor,
    outbound_http: OutboundHttpFactor,
    key_value: KeyValueFactor,
}

#[tokio::test(flavor = "multi_thread")]
async fn smoke_test_works() -> anyhow::Result<()> {
    let mut factors = Factors {
        wasi: WasiFactor::new(DummyFilesMounter),
        variables: VariablesFactor::default(),
        outbound_networking: OutboundNetworkingFactor,
        outbound_http: OutboundHttpFactor,
        key_value: KeyValueFactor::default(),
    };

    factors.variables.add_provider_type(StaticVariables)?;

    factors.key_value.add_store_type(TestSpinKeyValueStore)?;

    let locked = spin_loader::from_file(
        "tests/smoke-app/spin.toml",
        spin_loader::FilesMountStrategy::Direct,
        None,
    )
    .await?;
    let app = App::inert(locked);

    let engine = wasmtime::Engine::new(wasmtime::Config::new().async_support(true))?;
    let mut linker = wasmtime::component::Linker::new(&engine);

    factors.init(&mut linker).unwrap();

    let configured_app = factors.configure_app(app, TestSource)?;
    let data = factors.build_store_data(&configured_app, "smoke-app")?;

    assert_eq!(
        data.variables
            .resolver()
            .resolve("smoke-app", "other".try_into().unwrap())
            .await
            .unwrap(),
        "<other value>"
    );

    let mut store = wasmtime::Store::new(&engine, data);

    let component = configured_app.app().components().next().unwrap();
    let wasm_path = component
        .source()
        .content
        .source
        .as_deref()
        .unwrap()
        .strip_prefix("file://")
        .unwrap();
    let wasm_bytes = std::fs::read(wasm_path)
        .with_context(|| format!("wasm binary not found at '{wasm_path}'. Did you remember to run `spin build` in the `smoke-app` directory?"))?;
    let component_bytes = spin_componentize::componentize_if_necessary(&wasm_bytes)?;
    let component = wasmtime::component::Component::new(&engine, component_bytes)?;
    let instance = linker.instantiate_async(&mut store, &component).await?;

    // Invoke handler
    let req = http::Request::get("/").body(Default::default()).unwrap();
    let mut wasi_http_view =
        spin_factor_outbound_http::get_wasi_http_view::<Factors>(store.data_mut())?;
    let request = wasi_http_view.new_incoming_request(req)?;
    let (response_tx, response_rx) = tokio::sync::oneshot::channel();
    let response = wasi_http_view.new_response_outparam(response_tx)?;
    drop(wasi_http_view);

    let guest = wasmtime_wasi_http::proxy::Proxy::new(&mut store, &instance)?;
    let call_task = tokio::spawn(async move {
        guest
            .wasi_http_incoming_handler()
            .call_handle(&mut store, request, response)
            .await
    });
    let resp_task = tokio::spawn(async {
        let resp = response_rx.await.unwrap().unwrap();
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        eprintln!("Response: {body:?}");
    });
    let (call_res, resp_res) = tokio::join!(call_task, resp_task);
    let _ = call_res?;
    resp_res?;
    Ok(())
}

struct TestSource;

impl RuntimeConfigSource for TestSource {
    fn factor_config_keys(&self) -> impl IntoIterator<Item = &str> {
        [spin_factor_variables::RuntimeConfig::KEY]
    }

    fn get_factor_config<T: serde::de::DeserializeOwned>(
        &self,
        key: &str,
    ) -> anyhow::Result<Option<T>> {
        let Some(table) = toml::toml! {
            [[variable_provider]]
            type = "static"
            [variable_provider.values]
            foo = "bar"

            [key_value_store.default]
            type = "spin"
        }
        .remove(key) else {
            return Ok(None);
        };
        let config = table.try_into()?;
        Ok(Some(config))
    }
}

struct TestSpinKeyValueStore;

impl MakeKeyValueStore for TestSpinKeyValueStore {
    const RUNTIME_CONFIG_TYPE: &'static str = "spin";

    type RuntimeConfig = TestSpinKeyValueRuntimeConfig;

    type StoreManager = KeyValueSqlite;

    fn make_store(
        &self,
        runtime_config: Self::RuntimeConfig,
    ) -> anyhow::Result<Self::StoreManager> {
        let location = match runtime_config.path {
            Some(_) => {
                // TODO(lann): need state_dir to derive default store path
                bail!("spin key value runtime config not implemented")
            }
            None => DatabaseLocation::InMemory,
        };
        Ok(KeyValueSqlite::new(location))
    }
}

#[derive(Deserialize)]
struct TestSpinKeyValueRuntimeConfig {
    path: Option<PathBuf>,
}
