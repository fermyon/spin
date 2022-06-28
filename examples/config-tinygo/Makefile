.PHONY: build
build:
	tinygo build -wasm-abi=generic -target=wasi -gc=leaking -o main.wasm main.go

.PHONY: serve
serve:
	spin -vvvvv up --file spin.toml
