# Integration test

## Dependencies

Please add following dependencies to your PATH before running `make test-integration`.

* [bindle-server](https://github.com/deislabs/bindle)
* [nomad](https://github.com/hashicorp/nomad)
* [Hippo.Web](https://github.com/deislabs/hippo)

# E2E tests for spin

The goal of these tests is to ensure that spin continues to work with existing apps/examples/templates and there are no regressions introduced as we add more functionality or refactor existing code.

These tests work with following deployments models:

- spin up
- Fermyon Cloud
- Local platform (WIP)

## Important files and their function

`crates/e2e-testing`     - All the test framework/utilities that are required for `e2e-tests`
`tests/testcases/mod.rs` - All the testcase definitions should be added here.
`tests/spinup_tests.rs`  - All tests that we want to run with `Spin Up` should be added here
`tests/cloud_tests.rs`   - All tests that we want to run with `Fermyon Cloud` should be added here
`tests/testcases/<dirs>` - The testcases which require corresponding `spin app` pre-created should be added here


## Writing new testcase

### using pre-existing code scenario

Let us say we want to add a testcase `foo-env-test` for a specific scenario for which you have already created a `spin app`. Following steps are required to make this happen

1. You can add the existing app code `tests/testcases/foo-env-test`.
2. Add a new function `pub async fn foo_env_works(controller: &dyn Controller)` in `tests/testcases/mod.rs` as follows:

```rust

pub async fn foo_env_works(controller: &dyn Controller) {
    fn checks(app: &AppInstance) -> Result<()> {
        assert_http_request(
            get_url(app.metadata.base.as_str(), "/echo").as_str(),
            200,
            &[],
            Some("foo-env"),
        )?;

        Ok(())
    }

    let tc = TestCase {
        name: "foo-env-test".to_string(),
        //the appname should be same as dir where this app exists
        appname: "foo-env-test".to_string(),
        template: None,
        template_install_args: None,
        assertions: checks,
        plugins: None,
        deploy_args: None,
        skip_conditions: None,
        pre_build_hooks: None,
    };

    tc.run(controller).await.unwrap()
}

```

3. Add the testcase to `tests/spinup_tests.rs` (and to `tests/cloud_tests.rs` if want to run with `Fermyon Cloud` also) as follows:


```rust
#[tokio::test]
async fn foo_env_works() {
    testcases::foo_env_works(CONTROLLER).await
}
```

4. Run the tests locally to verify

```
docker build -t spin-e2e-tests .
docker run --rm -it docker.io/library/spin-e2e-tests 
```

### using a template
---------------------

Let us say we want to add a testcase for a new template `foo-bar`. Following steps are required to make this happen

1. Write a function `pub async fn foo_bar_works(controller: &dyn Controller)` as follows:

```rust
pub async fn foo_bar_works(controller: &dyn Controller) {
    fn checks(app: &AppInstance) -> Result<()> {
        return assert_http_request(
            app.metadata.base.as_str(),
            200,
            &[],
            Some("Hello foo-bar!\n"),
        );
    }

    let tc = TestCase {
        name: "foo-bar template".to_string(),
        appname: "foo-bar-test".to_string(),
        // this should be the name of the template used to 
        // create new app using `spin new <template-name> <app-name>
        template: Some("foo-bar".to_string()),
        template_install_args: None,
        assertions: checks,
        plugins: None,
        deploy_args: None,
        skip_conditions: None,
        pre_build_hooks: None,
    };

    tc.run(controller).await.unwrap();
}

```


2. Add the testcase to `tests/spinup_tests.rs` (and to `tests/cloud_tests.rs` if want to run with `Fermyon Cloud` also) as follows:

```rust
#[tokio::test]
async fn foo_bar_works() {
    testcases::foo_bar_works(CONTROLLER).await
}
```

3. Run the tests locally to verify

```
docker build -t spin-e2e-tests .
docker run --rm -it docker.io/library/spin-e2e-tests 
```

## Configure test to skip on `Fermyon cloud`
---------------------------------------------

Sometime there may be a functionality which works with `spin up` but not available yet on `Fermyon cloud`, we can skip such tests from running on `Fermyon cloud` by adding `skip_conditions` to `Testcase` as follows


```
framework.Testcase{
    name:       "headers-dynamic-env-test".to_string(),
    .
    .
    skip_conditions: Some(vec![SkipCondition {
        env: cloud_controller::NAME.to_string(),
        reason: "--env is not available on Fermyon cloud".to_string(),
    }]),
}
```