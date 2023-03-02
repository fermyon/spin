# Integration test

## Dependencies

Please add following dependencies to your PATH before running `make test-integration`.

* [bindle-server](https://github.com/deislabs/bindle)
* [nomad](https://github.com/hashicorp/nomad)
* [Hippo.Web](https://github.com/deislabs/hippo)

# E2E tests for spin

The goal of these tests is to ensure that spin continues to work with existing apps/examples/templates and there are no regressions introduced as we add more functionality or refactor existing code.

## How to run e2e tests

```sh
## go to root dir of the project, e2e-tests.Dockerfile is located there
make test-spin-up
```

## How to use `spin` binary with your local changes

By default, tests use the canary build of `spin` downloaded at the docker image creation time. If you want to test it with your changes, you can use the environment variable E2E_BUILD_SPIN=true

```sh
E2E_BUILD_SPIN=true make test-spin-up
```

## Important files and their function

* `crates/e2e-testing`     - All the test framework/utilities that are required for `e2e-tests`
* `tests/testcases/mod.rs` - All the testcase definitions should be added here.
* `tests/spinup_tests.rs`  - All tests that we want to run with `Spin Up` should be added here
* `tests/testcases/<dirs>` - The testcases which require corresponding `spin app` pre-created should be added here

## Key concepts and types

### [trait Controller](../crates/e2e-testing/src/controller.rs#L12)

This defines a trait which can be implemented by different deployment models (e.g. [`spin up`](../crates/e2e-testing/src/spin_controller.rs#L15) or `Fermyon Cloud`). Using this we can reuse the same testcases, which can be executed against these different deployment models (e.g. they may choose to have different way to start/stop the spin apps.)

### [TestCase](../crates/e2e-testing/src/testcase.rs#L22)

This helps us configure the different scenarios/steps which are required for running a specific test app. For example, [TestCase.trigger_type](../crates/e2e-testing/src/testcase.rs#L42) indicates the `trigger` a particular test app uses and [TestCase.plugins](../crates/e2e-testing/src/testcase.rs#L53) indicates which prerequisite [`plugins`](https://developer.fermyon.com/spin/plugin-authoring) are required to run this test app.

Additionally, [TestCase.assertions](../crates/e2e-testing/src/testcase.rs#L68) is a dynamic function to run testcase-specific assertions. During execution, the assertions function is called with the input parameters of `AppMetadata` as well as handles to the `stdlog/stderr` logs streams. The idea is that inside this function you would trigger your app (`http` or `redis` etc) and then verify if the trigger was successful by verifying either `http response` or `stdout/stderr`.

A basic assertion function for `http-trigger`

```rust
async fn checks(
        metadata: AppMetadata,
        _: Option<BufReader<ChildStdout>>,
        _: Option<BufReader<ChildStderr>>,
    ) -> Result<()> {
        assert_http_response(metadata.base.as_str(), 200, &[], Some("Hello Fermyon!\n")).await
}   
```

and for `redis-trigger`

```rust
async fn checks(
    _: AppMetadata,
    _: Option<BufReader<ChildStdout>>,
    stderr_stream: Option<BufReader<ChildStderr>>,
) -> Result<()> {
    //TODO: wait for spin up to be ready dynamically
    sleep(Duration::from_secs(10)).await;

    utils::run(vec!["redis-cli", "-u", "redis://redis:6379", "PUBLISH", "redis-go-works-channel", "msg-from-go-channel",], None, None)?;

    let stderr = utils::get_output_from_stderr(stderr_stream, Duration::from_secs(5)).await?;
    let expected_logs = vec!["Payload::::", "msg-from-go-channel"];

    assert!(expected_logs.iter().all(|item| stderr.contains(&item.to_string())));

    Ok(())
}

```

### [AppInstance](../crates/e2e-testing/src/controller.rs#L34)

This object holds the information about the app running as part of the testcase, e.g. it has details of routes for verifying `http trigger`-based apps and has handles to `stdout/stderr` log streams to assert the log messages printed by `redis trigger` templates.

It also holds a handle to the OS process which was started during the testcase execution. The testcase stops the process after the execution completes using the `controller.stop` method. This gives the control of how an app is run/stopped to the implementer of the specific deployment models.

## Writing new testcase

### using pre-existing code scenario

Let us say we want to add a testcase `foo-env-test` for a specific scenario for which you have already created a `spin app`. Following steps are required to make this happen

1. You can add the existing app code `tests/testcases/foo-env-test`.
2. Add a new function `pub async fn foo_env_works(controller: &dyn Controller)` in `tests/testcases/mod.rs` as follows:

```rust

pub async fn foo_env_works(controller: &dyn Controller) {
    async fn checks(
            metadata: AppMetadata,
            _: Option<BufReader<ChildStdout>>,
            _: Option<BufReader<ChildStderr>>,) -> Result<()> {
        assert_http_response(
            get_url(metadata.base.as_str(), "/echo").as_str(),
            200,
            &[],
            Some("foo-env"),
        )?;

        Ok(())
    }

        let tc = TestCaseBuilder::default()
        .name("foo-env-test".to_string())
        //the appname should be same as dir where this app exists
        .appname(Some("foo-env-test".to_string()))
        .template(None)
        .assertions(
            |metadata: AppMetadata,
                stdout_stream: Option<BufReader<ChildStdout>>,
                stderr_stream: Option<BufReader<ChildStderr>>| {
                Box::pin(checks(metadata, stdout_stream, stderr_stream))
            },
        )
        .build()
        .unwrap();

    tc.run(controller).await.unwrap()
}

```

3. Add the testcase to `tests/spinup_tests.rs` as follows:


```rust
#[tokio::test]
async fn foo_env_works() {
    testcases::all::foo_env_works(CONTROLLER).await
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
    async fn checks(metadata: AppMetadata,
            _: Option<BufReader<ChildStdout>>,
            _: Option<BufReader<ChildStderr>>,) -> Result<()> {
        return assert_http_response(
            metadata.base.as_str(),
            200,
            &[],
            Some("Hello foo-bar!\n"),
        );
    }

        let tc = TestCaseBuilder::default()
        .name("foo-bar template".to_string())
        // for template based tests, appname is generated on the fly
        .appname(None)
        // this should be the name of the template used to 
        // create new app using `spin new <template-name> <app-name>
        .template("foo-bar".to_string())
        .assertions(
            |metadata: AppMetadata,
                stdout_stream: Option<BufReader<ChildStdout>>,
                stderr_stream: Option<BufReader<ChildStderr>>| {
                Box::pin(checks(metadata, stdout_stream, stderr_stream))
            },
        )
        .build()
        .unwrap();

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

```sh
## go to root dir of the project, e2e-tests.Dockerfile is located there
docker build -t spin-e2e-tests -f e2e-tests.Dockerfile .
docker compose -f e2e-tests-docker-compose.yml run e2e-tests
```
