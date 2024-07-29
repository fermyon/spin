use std::{path::PathBuf, sync::Arc};

use anyhow::Context;
use http_body_util::BodyExt;
use spin_app::App;
use spin_factor_key_value::{
    delegating_resolver::{RuntimeConfigResolver, StoreConfig},
    KeyValueFactor, MakeKeyValueStore,
};
use spin_factor_key_value_redis::RedisKeyValueStore;
use spin_factor_key_value_spin::{SpinKeyValueRuntimeConfig, SpinKeyValueStore};
use spin_factor_outbound_http::OutboundHttpFactor;
use spin_factor_outbound_networking::OutboundNetworkingFactor;
use spin_factor_variables::{
    spin_cli::{StaticVariables, VariableProviderConfiguration},
    VariablesFactor,
};
use spin_factor_wasi::{DummyFilesMounter, WasiFactor};
use spin_factors::{
    Factor, FactorRuntimeConfigSource, RuntimeConfigSourceFinalizer, RuntimeFactors,
};
use wasmtime_wasi_http::WasiHttpView;

#[derive(RuntimeFactors)]
struct Factors {
    wasi: WasiFactor,
    variables: VariablesFactor<VariableProviderConfiguration>,
    outbound_networking: OutboundNetworkingFactor,
    outbound_http: OutboundHttpFactor,
    key_value: KeyValueFactor,
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
    let mut key_value_resolver = RuntimeConfigResolver::default();
    let default_config =
        SpinKeyValueRuntimeConfig::default(Some(PathBuf::from("tests/smoke-app/.spin")));
    key_value_resolver.add_default_store(
        "default",
        StoreConfig {
            type_: SpinKeyValueStore::RUNTIME_CONFIG_TYPE.to_string(),
            config: toml::value::Table::try_from(default_config)?,
        },
    );
    key_value_resolver.register_store_type(SpinKeyValueStore::new(
        std::env::current_dir().context("failed to get current directory")?,
    ))?;
    key_value_resolver.register_store_type(RedisKeyValueStore)?;
    let key_value_resolver = Arc::new(key_value_resolver);

    let mut factors = Factors {
        wasi: WasiFactor::new(DummyFilesMounter),
        variables: VariablesFactor::default(),
        outbound_networking: OutboundNetworkingFactor,
        outbound_http: OutboundHttpFactor,
        key_value: KeyValueFactor::new(key_value_resolver.clone()),
    };

    factors.variables.add_provider_resolver(StaticVariables)?;

    let locked = spin_loader::from_file(
        "tests/smoke-app/spin.toml",
        spin_loader::FilesMountStrategy::Direct,
        None,
    )
    .await?;
    let app = App::new("test-app", locked);

    let engine = wasmtime::Engine::new(wasmtime::Config::new().async_support(true))?;
    let mut linker = wasmtime::component::Linker::new(&engine);

    factors.init(&mut linker)?;

    let source = TestSource { key_value_resolver };
    let configured_app = factors.configure_app(app, source.try_into()?)?;
    let builders = factors.prepare(&configured_app, "smoke-app")?;
    let state = factors.build_instance_state(builders)?;

    assert_eq!(
        state
            .variables
            .expression_resolver()
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

struct TestSource {
    key_value_resolver: Arc<RuntimeConfigResolver>,
}

impl TryFrom<TestSource> for FactorsRuntimeConfig {
    type Error = anyhow::Error;

    fn try_from(value: TestSource) -> Result<Self, Self::Error> {
        Self::from_source(value)
    }
}

impl FactorRuntimeConfigSource<KeyValueFactor> for TestSource {
    fn get_runtime_config(
        &mut self,
    ) -> anyhow::Result<Option<<KeyValueFactor as Factor>::RuntimeConfig>> {
        let config = toml::toml! {
            [other]
            type = "redis"
            url = "redis://localhost:6379"
        };
        self.key_value_resolver.resolve_from_toml(&Some(config))
    }
}

impl FactorRuntimeConfigSource<VariablesFactor<VariableProviderConfiguration>> for TestSource {
    fn get_runtime_config(
        &mut self,
    ) -> anyhow::Result<
        Option<<VariablesFactor<VariableProviderConfiguration> as Factor>::RuntimeConfig>,
    > {
        let config = toml::toml! {
            [[variable_provider]]
            type = "static"
            [variable_provider.values]
            foo = "bar"
        }
        .remove("variable_provider")
        .unwrap();
        Ok(Some(config.try_into()?))
    }
}

impl FactorRuntimeConfigSource<WasiFactor> for TestSource {
    fn get_runtime_config(
        &mut self,
    ) -> anyhow::Result<Option<<WasiFactor as Factor>::RuntimeConfig>> {
        Ok(None)
    }
}

impl FactorRuntimeConfigSource<OutboundNetworkingFactor> for TestSource {
    fn get_runtime_config(
        &mut self,
    ) -> anyhow::Result<Option<<OutboundNetworkingFactor as Factor>::RuntimeConfig>> {
        Ok(None)
    }
}

impl FactorRuntimeConfigSource<OutboundHttpFactor> for TestSource {
    fn get_runtime_config(
        &mut self,
    ) -> anyhow::Result<Option<<OutboundHttpFactor as Factor>::RuntimeConfig>> {
        Ok(None)
    }
}

impl RuntimeConfigSourceFinalizer for TestSource {
    fn finalize(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}
