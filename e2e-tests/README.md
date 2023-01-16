# e2e tests for spin

The goal of these tests is to ensure that spin continues to work with existing apps/examples/templates and there are no regressions introduced as we add more functionality or refactor existing code.

These tests work with following deployments models:

- spin up
- Fermyon Cloud
- Local platform (WIP)

## Writing new testcase

### using pre-existing code scenario


Let us say we want to add a testcase `foo-env-test` for a specific scenario. Following steps are required to make this happen

1. You can add directory that has code under `e2e-tests/tests/foo-env-test`.
2. Add a new file `e2e-tests/tests/foo_env_test.go`.
2. Write a function `fooEnvTestcase() framework.Testcase` as follows:

```go
package tests

import (
	"testing"

	"github.com/fermyon/spin/e2e-tests/internal/framework"
	"github.com/fermyon/spin/e2e-tests/internal/spin"
)

func fooEnvTestcase() framework.Testcase {
		return framework.Testcase{
		Name:       "foo-env-test",
		AppName:    "foo-env-test",
		DeployArgs: []string{"--env", "foo=bar"},
		SkipConditions: []framework.SkipCondition{
			{
				Env:    spin.FermyonCloud,
				Reason: "--env is not supported with spin deploy",
			},
		},
		SubTestsExecutor: func(t *testing.T, metadata *spin.Metadata) {
			routeBase, err := url.Parse(metadata.Base)
			require.Nil(t, err, "valid url for app route")

			assertHTTPRequest(t, routeBase.JoinPath("/env").String(), http.StatusOK, nil, "")
			assertHTTPRequest(t, routeBase.JoinPath("/env/foo").String(), http.StatusOK, map[string]string{"env_foo": "bar", "env_some_key": "some_value"}, "")
		},
	}
}

```

3. Add the testcase to `spin_test.go` as follows:


```bash
index 0609fb0..dfc5476 100644
--- a/e2e-tests/tests/spin_test.go
+++ b/e2e-tests/tests/spin_test.go
@@ -50,6 +50,7 @@ func testSpinTemplates(t *testing.T, controller spin.Controller) {
                headersDynamicEnvRoutesTestcase(),
+               fooEnvTestcase(),
        } {
                func(testcase framework.Testcase, t *testing.T) {
                        t.Run(testcase.Name, func(t *testing.T) {
```

4. Run the new test locally to verify

```
go test ./... -run ^TestSpinTemplatesUsingSpinUp/foo-env-test$
```

### using a template
---------------------

Let us say we want to add a testcase for a new template `foo-bar`. Following steps are required to make this happen

1. Add a new file `foo_bar_test.go` under `e2e-tests/tests`
2. Write a function `fooBarTestcase() framework.Testcase` as follows:

```go
package tests

import (
	"testing"

	"github.com/fermyon/spin/e2e-tests/internal/framework"
	"github.com/fermyon/spin/e2e-tests/internal/spin"
)

func fooBarTestcase() framework.Testcase {
	return framework.Testcase{
		Name:     "foo-bar-template",
		Template: "foo-bar",
		AppName:  "foo-bar-test",
		SubTestsExecutor: func(t *testing.T, metadata *spin.Metadata) {
			assertGetWorks(t, metadata.GetRouteWithName(metadata.AppName).RouteURL, "Hello Fermyon!\n")
		},
	}
}

```

3. Add the testcase to `spin_test.go` as follows:


```bash
index 0609fb0..dfc5476 100644
--- a/e2e-tests/tests/spin_test.go
+++ b/e2e-tests/tests/spin_test.go
@@ -50,6 +50,7 @@ func testSpinTemplates(t *testing.T, controller spin.Controller) {
                headersDynamicEnvRoutesTestcase(),
+               fooBarTestcase(),
        } {
                func(testcase framework.Testcase, t *testing.T) {
                        t.Run(testcase.Name, func(t *testing.T) {
```

4. Run the new test locally to verify

```
go test ./... -run ^TestSpinTemplatesUsingSpinUp/foo-bar-template$
```

### Configure test to skip on `Fermyon cloud`
---------------------------------------------

Sometime there may be a functionality which works with `spin up` but not supported yet on `Fermyon cloud`, we can skip such tests from running on `Fermyon cloud` by adding `SkipConditions` to `framework.Testcase` as follows


```
framework.Testcase{
    Name:       "headers-dynamic-env-test",
    .
    .
    SkipConditions: []framework.SkipCondition{
        {
            Env:    spin.FermyonCloud,
            Reason: "--env is not supported with Fermyon Cloud",
        },
    },
}
```