use spin_factors::SpinFactors;

#[derive(SpinFactors)]
struct Factors {}

fn main() -> anyhow::Result<()> {
    let mut factors = Factors {};

    let engine = wasmtime::Engine::default();
    let mut linker = wasmtime::component::Linker::new(&engine);
    factors.init(&mut linker).unwrap();

    let factors = Factors {
        // wasi: WasiFactor,
        // outbound_networking_factor: OutboundNetworkingFactor,
        // outbound_http_factor: OutboundHttpFactor,
    };
    let data = factors.build_data().unwrap();

    let mut store = wasmtime::Store::new(&engine, data);
    let component = wasmtime::component::Component::new(&engine, b"(component)").unwrap();
    let _instance = linker.instantiate(&mut store, &component).unwrap();
    Ok(())
}
