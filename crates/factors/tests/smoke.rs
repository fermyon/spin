use spin_factor_wasi::{preview1::WasiPreview1Factor, WasiFactor};
use spin_factors::SpinFactors;

#[derive(SpinFactors)]
struct Factors {
    wasi: WasiFactor,
    wasip1: WasiPreview1Factor,
}

fn main() -> anyhow::Result<()> {
    let mut factors = Factors {
        wasi: WasiFactor,
        wasip1: WasiPreview1Factor,
        // outbound_networking_factor: OutboundNetworkingFactor,
        // outbound_http_factor: OutboundHttpFactor,
    };

    let engine = wasmtime::Engine::default();
    let mut linker = wasmtime::component::Linker::new(&engine);
    let mut module_linker = wasmtime::Linker::new(&engine);

    factors
        .init(Some(&mut linker), Some(&mut module_linker))
        .unwrap();
    let data = factors.build_store_data().unwrap();

    let mut store = wasmtime::Store::new(&engine, data);

    let component = wasmtime::component::Component::new(&engine, b"(component)").unwrap();
    let _instance = linker.instantiate(&mut store, &component).unwrap();

    let module = wasmtime::Module::new(&engine, b"(module)").unwrap();
    let _module_instance = module_linker.instantiate(&mut store, &module).unwrap();

    Ok(())
}
