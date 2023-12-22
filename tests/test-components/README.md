# Test Components

Test components for use in runtime testing. Each test component has a README on what it tests. The components are checked into the repository so that users do not necessarily have to build them from source.

This crate will build all of the components as part of its `build.rs` build script. It then generates code in the lib.rs file which exposes the paths to the built components as constants. For example, for a component named foo-component, a `FOO_COMPONENT` const will be generated with the path to the built component binary.

Additionally, a helper function named `path` is generated that maps a package name to binary path for dynamic lookups.

## Building

This crate is built like a normal Rust crate: `cargo build`

## Contract

Test components have the following contract with the outside world:

* They do not look at the incoming request.
* If nothing errors a 200 with no body will be returned.
* If an error occurs a 500 with a body describing the error will be returned.

## Adapter support

Components can optionally be adapted using a preview 1 to preview 2 adapter (instead of relying on the Spin runtime to do so). 

The adapters can be found in the `adapters` directory. They come from the `wasmtime` project and can be downloaded here:

https://github.com/bytecodealliance/wasmtime/releases
