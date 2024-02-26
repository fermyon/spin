# Runtime tests

Runtime tests are a specific type of test for testing the runtime behavior of a Spin compliant runtime.

For the purposes of these tests, an "application" is a collection of the following things:
* A Spin compliant WebAssembly binary
* A spin.toml manifest
* Optional runtime-config.toml files

## What are runtime tests supposed to test and not test?

Runtime tests are meant to test the runtime functionality of Spin. In other words, they ensure that a valid combination of Spin manifest and some number of Spin compliant WebAssembly binaries perform in expected ways or fail in expected ways.

Runtime tests are not full end-to-end integration tests, and thus there are some things they do not concern themselves with including:
* Different arguments to the Spin CLI
* Failure cases that cause Spin not to start a running http server (e.g., malformed manifest, malformed WebAssembly binaries etc.)
* Bootstrapping WebAssembly modules into compliant WebAssembly components (e.g., turning Wasm modules created with JavaScript tooling into WebAssembly components using `js2wasm`)

## How do I run the tests?

The runtime tests can either be run as a library function (e.g., this is how they are run as part of Spin's test suite using `cargo test`), or they can be run stand alone using the `runtime-tests` crate's binary (i.e., running `cargo run` from this directory).

## How do I add a new test?

To add a new test you must add a new folder to the `tests` directory with at least a `spin.toml` manifest.

The manifest is actually a template that allows for a few values to be interpolated by the test runner. The interpolation happens through `%{key=value}` annotations where `key` is one of a limited number of keys the test runner supports. The supported keys are:

* `source`: The manifest can reference pre-built Spin compliant WebAssembly modules that can be found in the `test-components` folder in the Spin repo. The value is substituted for the name of the test component to be used. For example `%{source=sqlite}` will use the test-component named "sqlite" found in the `test-components` directory.
* `port`: The manifest can reference a port that has been exposed by a service (see the section on services below). For example, if the test runner sees `%{port=1234}` it will look for a service that exposes the guest port 1234 on some randomly assigned host port and substitute `%{port=1234}` for that randomly assigned port.

The test directory may additionally contain:
* an `error.txt` if the Spin application is expected to fail
* a `services` config file (more on this below)

### The testing protocol

The test runner will make a GET request against the `/` path. The component should either return a 200 if everything goes well or a 500 if there is an error. If an `error.txt` file is present, the Spin application must return a 500 with the body set to some error message that contains the contents of `error.txt`.

## When do tests pass?

A test will pass in the following conditions:
* The Spin web server returns a 200
* The Spin web server returns a 500 with a body that contains the same text inside of the test's error.txt file.