# Wasi HTTP (v0.2.0-rc-2023-11-10)

Tests the Wasi HTTP outgoing request handler specifically the 0.2.0-rc-2023-11-10 version.

The `wit` directory was copied from https://github.com/bytecodealliance/wasmtime/tree/v15.0.1/crates/wasi/wit and then modified to only include the parts actually used by this component.

## Expectations

This test component expects the following to be true:
* It has access to an HTTP server on localhost:8080 that accepts POST requests and returns the same bytes in the response body as in the request body.
