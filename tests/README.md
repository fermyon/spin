# Integration test

## Dependencies

Please add following dependencies to your PATH before running `make test-integration`.

* [bindle-server](https://github.com/deislabs/bindle)
* [nomad](https://github.com/hashicorp/nomad)
* [Hippo.Web](https://github.com/deislabs/hippo)

# E2E tests for spin

The goal of these tests is to ensure that spin continues to work with existing apps/examples/templates and there are no regressions introduced as we add more functionality or refactor existing code.

## How to run e2e tests

```
## go to root dir of the project, e2e-tests.Dockerfile is located there
docker build -t spin-e2e-tests -f e2e-tests.Dockerfile .
docker compose -f e2e-tests-docker-compose.yml run e2e-tests
```

## How to run e2e tests on aarch64
```
## go to root dir of the project, e2e-tests-aarch64.Dockerfile is located there
docker build -t spin-e2e-tests -f e2e-tests-aarch64.Dockerfile .
MYSQL_IMAGE=arm64v8/mysql:8.0.32 REDIS_IMAGE=arm64v8/redis:6.0-alpine3.17 docker compose         \
    -f e2e-tests-docker-compose.yml run                 \
    e2e-tests
```

## How to use `spin` binary with your local changes

By default tests use the canary build of `spin` downloaded at docker image creation time. If you want to test it with your changes, you can use `--build-arg BUILD_SPIN=true`

```
docker build --build-arg BUILD_SPIN=true -t spin-e2e-tests -f e2e-tests.Dockerfile .
docker compose                                          \
    -f e2e-tests-docker-compose.yml run                 \
    e2e-tests
```

## Important files and their function

`crates/e2e-testing`     - All the test framework/utilities that are required for `e2e-tests`
`tests/testcases/mod.rs` - All the testcase definitions should be added here.
`tests/spinup_tests.rs`  - All tests that we want to run with `Spin Up` should be added here
`tests/testcases/<dirs>` - The testcases which require corresponding `spin app` pre-created should be added here


## Writing new testcase

### using pre-existing code scenario

Let us say we want to add a testcase `foo-env-test` for a specific scenario for which you have already created a `spin app`. Following steps are required to make this happen

1. You can add the existing app code `tests/testcases/foo-env-test`.
2. Add a new function `pub async fn foo_env_works(controller: &dyn Controller)` in `tests/testcases/mod.rs` as follows:

```rust

pub async fn foo_env_works(controller: &dyn Controller) {
    fn checks(metadata: &AppMetadata) -> Result<()> {
        assert_http_response(
            get_url(metadata.base.as_str(), "/echo").as_str(),
            200,
            &[],
            Some("foo-env"),
        )?;

        Ok(())
    }

    let tc = TestCase {
        name: "foo-env-test".to_string(),
        //the appname should be same as dir where this app exists
        appname: Some("foo-env-test".to_string()),
        template: None,
        template_install_args: None,
        assertions: checks,
        plugins: None,
        deploy_args: None,
        pre_build_hooks: None,
    };

    tc.run(controller).await.unwrap()
}

```

3. Add the testcase to `tests/spinup_tests.rs` as follows:


```rust
#[tokio::test]
async fn foo_env_works() {
    testcases::foo_env_works(CONTROLLER).await
}
```

4. Run the tests locally to verify

```
## go to root dir of the project, e2e-tests.Dockerfile is located there
docker build -t spin-e2e-tests -f e2e-tests.Dockerfile .
docker compose -f e2e-tests-docker-compose.yml run e2e-tests
```

### using a template
---------------------

Let us say we want to add a testcase for a new template `foo-bar`. Following steps are required to make this happen

1. Write a function `pub async fn foo_bar_works(controller: &dyn Controller)` as follows:

```rust
pub async fn foo_bar_works(controller: &dyn Controller) {
    fn checks(metadata: &AppMetadata) -> Result<()> {
        return assert_http_response(
            metadata.base.as_str(),
            200,
            &[],
            Some("Hello foo-bar!\n"),
        );
    }

    let tc = TestCase {
        name: "foo-bar template".to_string(),
        // for template based tests, appname is generated on the fly
        appname: None,
        // this should be the name of the template used to 
        // create new app using `spin new <template-name> <app-name>
        template: Some("foo-bar".to_string()),
        template_install_args: None,
        assertions: checks,
        plugins: None,
        deploy_args: None,
        pre_build_hooks: None,
    };

    tc.run(controller).await.unwrap();
}

```


2. Add the testcase to `tests/spinup_tests.rs` as follows:

```rust
#[tokio::test]
async fn foo_bar_works() {
    testcases::foo_bar_works(CONTROLLER).await
}
```

3. Run the tests locally to verify

```
## go to root dir of the project, e2e-tests.Dockerfile is located there
docker build -t spin-e2e-tests -f e2e-tests.Dockerfile .
docker compose -f e2e-tests-docker-compose.yml run e2e-tests
```
