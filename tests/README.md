# Testing

Spin is tested through several classes of tests that aim to build confidence in the correctness of Spin from multiple angles:

* Unit tests
* Runtime tests
* Integration tests

## Unit tests

Spin is composed of a many different crates that each test their individual functionality using normal Rust unit tests. You can run these tests like you would for any Rust based project using Cargo:

```bash
cargo test -p $CRATE_NAME
```

## Runtime tests

Runtime tests are meant to test Spin compliant runtimes to ensure that they conform to expected runtime behavior.

The runtime tests are handled through the `runtime-tests` support crate. See the README there for more information.

You can run runtime tests like so:

```bash
cargo test runtime_tests -F extern-dependencies-tests
```

# Integration tests

Integration tests are meant to test anything that cannot be tested through some other testing mechanism usually because the scenario under test is complicated and involves the interaction between many different subsystems.  Historically, integration tests have been a landing pad for experimentation around testing that have eventually been turned into their own class of tests. 

You can run integration tests like so:
```bash
make test-integration
```

Note that this also runs the runtime tests as well.

This will not run the full integration test suite, but a subset that only relies the presence of Rust and Python toolchains. The full integration test suite runs tests that rely on Docker and some additional compiler toolchains (e.g., Swift, Zig, etc.). Eventually, we want to only require the presence of Docker, but we're not quite there yet. You can run the full test suite like so:

```bash
make test-integration-full
```
