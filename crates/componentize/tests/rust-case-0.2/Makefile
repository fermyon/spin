spin-abi-conformance.wasm: target/wasm32-wasi/release/spin-rust.wasm
	cp $< $@

target/wasm32-wasi/release/spin-rust.wasm: src/main.rs Cargo.toml
	cargo build --target wasm32-wasi --release
