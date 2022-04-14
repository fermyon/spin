# tinygo requires go version 1.15 through 1.17
# Go binary and GOROOT to select
GO ?= go
export GOROOT = $(shell $(GO) env GOROOT)

.PHONY: build
build:
	tinygo build -wasm-abi=generic -target=wasi -gc=leaking -o main.wasm main.go

.PHONY: serve
serve:
	RUST_LOG=spin=trace,wasi_outbound_http=trace spin up --file spin.toml
