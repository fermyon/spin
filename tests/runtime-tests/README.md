# Runtime Tests

The runtime tests ensure that Spin can properly run applications. 

For the purposes of these tests, an "application" is a collection of the following things:
* A Spin compliant WebAssembly binary
* A spin.toml manifest
* A list of arguments to be passed to `spin up`
* A collection of a files needed for running `spin up` (e.g., a migration file for passing to the `--sqlite` argument)

## How do I run the tests?

The runtime tests can either be run as a library function (e.g., this is how they are run as part of Spin's test suite using `cargo test`) or they can be run stand alone using the `runtime-tests` crate's binary (i.e., running `cargo run` from this directory).

## How do I add a new test?

To add a new test you must add a new folder to the `tests` directory with at least a `spin.toml` manifest. 

The manifest can reference pre-built Spin compliant WebAssembly modules that can be found in the `test-components` folder in the Spin repo. It does so by using the `{{$NAME}}` where `$NAME` is substituted for the name of the test component to be used. For example `{{sqlite}}` will use the test-component named "sqlite" found in the `test-components` directory.

Optionally, an `args` file can be added with newline separated arguments to be passed to the `spin up` invocation, and a `data` directory can be added which will have all of its contents copied into the temporary folder where `spin up` will be run.

## When do tests pass?

A test will pass in the following conditions:
* The Spin web server returns a 200
* The Spin web server returns a 500 with a body that contains the same text inside of the test's error.txt file.
