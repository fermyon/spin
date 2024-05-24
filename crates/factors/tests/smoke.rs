use spin_app::App;
use spin_factor_outbound_networking::OutboundNetworkingFactor;
use spin_factor_wasi::{preview1::WasiPreview1Factor, DummyFilesMounter, WasiFactor};
use spin_factors::SpinFactors;

#[derive(SpinFactors)]
struct Factors {
    wasi: WasiFactor,
    wasip1: WasiPreview1Factor,
    outbound_networking_factor: OutboundNetworkingFactor,
}

fn main() -> anyhow::Result<()> {
    let mut factors = Factors {
        wasi: WasiFactor::new(DummyFilesMounter),
        wasip1: WasiPreview1Factor,
        outbound_networking_factor: OutboundNetworkingFactor,
        // outbound_http_factor: OutboundHttpFactor,
    };

    let locked = serde_json::from_value(serde_json::json!({
        "spin_locked_version": 1,
        "triggers": [],
        "components": [{
            "id": "test",
            "source": {
                "content_type": "application/wasm",
                "content": {"inline": "KGNvbXBvbmVudCk="}
            }
        }]
    }))
    .unwrap();
    let app = App::inert(locked);

    let engine = wasmtime::Engine::default();
    let mut linker = wasmtime::component::Linker::new(&engine);
    let mut module_linker = wasmtime::Linker::new(&engine);

    factors
        .init(Some(&mut linker), Some(&mut module_linker))
        .unwrap();

    let configured_app = factors.configure_app(app).unwrap();
    let data = factors.build_store_data(&configured_app, "test").unwrap();

    let mut store = wasmtime::Store::new(&engine, data);

    let component = wasmtime::component::Component::new(&engine, b"(component)").unwrap();
    let _instance = linker.instantiate(&mut store, &component).unwrap();

    let module = wasmtime::Module::new(&engine, b"(module)").unwrap();
    let _module_instance = module_linker.instantiate(&mut store, &module).unwrap();

    Ok(())
}
