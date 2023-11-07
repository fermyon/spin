# Runtime Tests

The runtime tests ensure that Spin can properly run applications. 

For the purposes of these tests, an "application" is a collection of the following things:
* A Spin compliant WebAssembly binary
* A spin.toml manifest
* A list of arguments to be passed to `spin up`
* A collection of a files needed for running `spin up` (e.g., a migration file for passing to the `--sqlite` argument)

## Adding a new test

To add a new test you must add a new folder to the `tests` directory with at least a `spin.toml` manifest. 

The manifest can reference pre-built Spin compliant WebAssembly modules that can be found in the `test-components` folder in the Spin repo. It does so by using the `{{$NAME}}` where `$NAME` is substituted for the name of the test component to be used. For example `{{sqlite}}` will use the test-component named "sqlite" found in the `test-components` directory.

Optionally, an `args` file can be added with newline separated arguments to be passed to the `spin up` invocation, and a `data` directory can be added which will have all of its contents copied into the temporary folder where `spin up` will be run.

## Running the tests

Running the tests is as easy as running the `runtime-tests` binary. In this directory run:

```bash
cargo run
```