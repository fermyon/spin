.PHONY: build
build:
	tinygo build -target=wasi -gc=leaking -o main.wasm main.go
	wasm-tools component embed ../../../../../wit/preview2 --world reactor main.wasm -o main.wasm