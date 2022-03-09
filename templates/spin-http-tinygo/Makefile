.PHONY: build
build:
	tinygo build -wasm-abi=generic -target=wasi -o main.wasm main.go

. PHONY: serve
serve:
	RUST_LOG=spin=trace spin up --file spin.toml
