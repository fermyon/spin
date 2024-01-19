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
cargo test runtime_tests -F e2e-tests
```

# Integration tests

Integration tests are meant to test anything that cannot be tested through some other testing mechanism usually because the scenario under test is complicated and involves the interaction between many different subsystems.  Historically, integration tests have been a landing pad for experimentation around testing that have eventually been turned into their own class of tests. 

Currently, integration tests are split between two different modules that will soon be combined into one: `integration_tests` and `spinup_tests`.

You can run integration tests like so:
```bash
make test-integration
make test-spin-up
```
