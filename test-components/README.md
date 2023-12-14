# Test Components

Test components for use in runtime testing. Each test component has a README on what it tests. The components are checked into the repository so that users do not necessarily have to build them from source.

## Building 

Each component is generally built like so:

```
cargo b --target=wasm32-wasi
```

Additionally, to prevent bloat, the components are run through `wasm-tools strip`.

## Contract

Test components have the following contract with the outside world:

* They do not look at the incoming request.
* If nothing errors a 200 with no body will be returned.
* If an error occurs a 500 with a body describing the error will be returned.
