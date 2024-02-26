# Adapters

The componentize process uses adapters to adapt plain wasm modules to wasi preview 2 compatible wasm components. There are three adapters that are built and stored as wasm binaries in this repository:

* The upstream wasi preview1 adapters for both commands and reactors for use with newer versions of wit-bindgen (v0.5 and above).
    * These are currently [the wasmtime 18.0.1 release](https://github.com/bytecodealliance/wasmtime/releases/tag/v18.0.1).
* A modified adapter that has knowledge of Spin APIs for use with v0.2 of wit-bindgen which has a different ABI than newer wit-bindgen based modules.
    * This is currently built using commit [603fb3e](https://github.com/rylev/wasmtime/commit/603fb3e14fb0eb7468b832711fee5ff7e7ce7012) on the github.com/rylev/wasmtime fork of wasmtime.
    * You can see a diff between the upstream wasmtime 18.0.1 compatible adapter and this custom adapter [here](https://github.com/bytecodealliance/wasmtime/compare/release-18.0.0...rylev:wasmtime:v18.0.1-spin).

