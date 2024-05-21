use spin_factor_wasi::WasiFactor;
use spin_factors::SpinFactors;

#[derive(SpinFactors)]
struct Factors {
    wasi: WasiFactor,
}

fn main() -> anyhow::Result<()> {
    let engine = wasmtime::Engine::default();
    let mut linker = wasmtime::component::Linker::new(&engine);

    let mut factors = Factors {
        wasi: WasiFactor,
        // outbound_networking_factor: OutboundNetworkingFactor,
        // outbound_http_factor: OutboundHttpFactor,
    };
    factors.init(&mut linker).unwrap();
    let data = factors.build_store_data().unwrap();

    let mut store = wasmtime::Store::new(&engine, data);
    let component = wasmtime::component::Component::new(&engine, b"(component)").unwrap();
    let _instance = linker.instantiate(&mut store, &component).unwrap();
    Ok(())
}
