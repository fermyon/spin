# spin-componentize

This library converts a Spin module to a
[component](https://github.com/WebAssembly/component-model/).

Note that although the world specifies both `inbound-redis` and `inbound-http`
exports, `spin-componentize` will only export either or both according to what
the original module exported.

## Building

This crate requires a [Rust](https://rustup.rs/) installation v1.68 or later and a couple of Wasm targets:

```shell
rustup target add wasm32-wasip1
rustup target add wasm32-unknown-unknown
```

Note that this is currently only a library and does not yet have a CLI interface, although that 
would be easy to add if desired.

## Testing

To test whether the spin componentize process produces wasm components that can be used with wasmtime, we run "abi conformance" testing. These tests are run with a plain `cargo test` invocation.

## Wit and Adapters

spin-componentize and the abi conformance tests use component adapters built from wasmtime.

See the [adapters README](./adapters/README.md) for more information.
