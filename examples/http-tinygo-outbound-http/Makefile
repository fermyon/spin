.PHONY: build
build:
	tinygo build -wasm-abi=generic -target=wasi -gc=leaking -no-debug -o main.wasm main.go

. PHONY: serve
serve:
	RUST_LOG=spin=trace,wasi_outbound_http=trace spin up --file spin.toml
