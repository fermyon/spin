# Testing Framework

The testing framework is a general framework for running tests against a Spin compliant runtime. The testing framework includes the ability to start up dependent services (e.g., Redis or MySQL).

## Services

Services allow for tests to be run against external sources. The service definitions can be found in the 'services' directory. Each test directory contains a 'services' file that configures the tests services. Each line of the services file should contain the name of a services file that needs to run. For example, the following 'services' file will run the `tcp-echo.py` service:

```txt
tcp-echo
```

Each service is run under a file lock meaning that all other tests that require that service must wait until the current test using that service has finished.

The following service types are supported:
* Python services (a python script ending in the .py file extension)
* Docker services (a docker file ending in the .Dockerfile extension)

When looking to add a new service, always prefer the Python based service as it's generally much quicker and lighter weight to run a Python script than a Docker container. Only use Docker when the service you require is not possible to achieve in cross platform way as a Python script.

### Signaling Service Readiness

Services can signal that they are ready so that tests aren't run against them until they are ready:

* Python: Python services signal they are ready by printing `READY` to stdout.
* Docker: Docker services signal readiness by exposing a Docker health check in the Dockerfile (e.g., `HEALTHCHECK --start-period=4s --interval=1s CMD  /usr/bin/mysqladmin ping --silent`)

### Exposing Ports

Both Docker and Python based services can expose some logical port number that will be mapped to a random free port number at runtime.

* Python: Python based services can do this by printing `PORT=($PORT1, $PORT2)` to stdout where the $PORT1 is the logical port the service exposes and $PORT2 is the random port actually being exposed (e.g., `PORT=(80, 59392)`)
* Docker: Docker services can do this by exposing the port in their Dockerfile (e.g., `EXPOSE 3306`)
The runtime tests ensure that Spin can properly run applications. 

For the purposes of these tests, an "application" is a collection of the following things:
* A Spin compliant WebAssembly binary
* A spin.toml manifest
* Optional runtime-config.toml files

## Runtime tests

Runtime tests are a specific type of test for testing the runtime behavior of a Spin compliant runtime.

### What are runtime tests supposed test and not test?

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
