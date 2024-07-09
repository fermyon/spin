use std::path::PathBuf;

use anyhow::Context;
use http_body_util::BodyExt;
use spin_app::App;
use spin_factor_key_value::{
    delegating_resolver::DelegatingRuntimeConfigResolver, KeyValueFactor, MakeKeyValueStore,
};
use spin_factor_key_value_redis::RedisKeyValueStore;
use spin_factor_key_value_spin::{SpinKeyValueRuntimeConfig, SpinKeyValueStore};
use spin_factor_outbound_http::OutboundHttpFactor;
use spin_factor_outbound_networking::OutboundNetworkingFactor;
use spin_factor_variables::{StaticVariables, VariablesFactor};
use spin_factor_wasi::{DummyFilesMounter, WasiFactor};
use spin_factors::{FactorRuntimeConfig, RuntimeConfigSource, RuntimeFactors};
use wasmtime_wasi_http::WasiHttpView;

#[derive(RuntimeFactors)]
struct Factors {
    wasi: WasiFactor,
    variables: VariablesFactor,
    outbound_networking: OutboundNetworkingFactor,
    outbound_http: OutboundHttpFactor,
    key_value: KeyValueFactor,
    redis: OutboundRedisFactor,
}

struct Data {
    factors_instance_state: FactorsInstanceState,
    _other_data: usize,
}

impl AsMut<FactorsInstanceState> for Data {
    fn as_mut(&mut self) -> &mut FactorsInstanceState {
        &mut self.factors_instance_state
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn smoke_test_works() -> anyhow::Result<()> {
    let mut key_value_resolver = DelegatingRuntimeConfigResolver::default();
    let default_config =
        SpinKeyValueRuntimeConfig::default(Some(PathBuf::from("tests/smoke-app/.spin")));
    key_value_resolver.add_default_store(
        "default",
        SpinKeyValueStore::RUNTIME_CONFIG_TYPE,
        toml::value::Table::try_from(default_config)?,
    );
    key_value_resolver.add_store_type(SpinKeyValueStore::new(None)?)?;
    key_value_resolver.add_store_type(RedisKeyValueStore)?;

    let mut factors = Factors {
        wasi: WasiFactor::new(DummyFilesMounter),
        variables: VariablesFactor::default(),
        outbound_networking: OutboundNetworkingFactor,
        outbound_http: OutboundHttpFactor,
        key_value: KeyValueFactor::new(key_value_resolver),
    };

    factors.variables.add_provider_type(StaticVariables)?;

    let locked = spin_loader::from_file(
        "tests/smoke-app/spin.toml",
        spin_loader::FilesMountStrategy::Direct,
        None,
    )
    .await?;
    let app = App::inert(locked);

    let engine = wasmtime::Engine::new(wasmtime::Config::new().async_support(true))?;
    let mut linker = wasmtime::component::Linker::new(&engine);

    factors.init::<Data>(&mut linker).unwrap();

    let configured_app = factors.configure_app(app, TestSource)?;
    let builders = factors.prepare(&configured_app, "smoke-app")?;
    let state = factors.build_instance_state(builders)?;

    assert_eq!(
        state
            .variables
            .resolver()
            .resolve("smoke-app", "other".try_into().unwrap())
            .await
            .unwrap(),
        "<other value>"
    );

    let data = Data {
        factors_instance_state: state,
        _other_data: 1,
    };

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
    let mut wasi_http = OutboundHttpFactor::get_wasi_http_impl(store.data_mut().as_mut()).unwrap();
    let request = wasi_http.new_incoming_request(req)?;
    let (response_tx, response_rx) = tokio::sync::oneshot::channel();
    let response = wasi_http.new_response_outparam(response_tx)?;
    drop(wasi_http);

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

            [key_value_store.other]
            type = "redis"
            url = "redis://localhost:6379"
        }
        .remove(key) else {
            return Ok(None);
        };
        let config = table.try_into()?;
        Ok(Some(config))
    }
}
