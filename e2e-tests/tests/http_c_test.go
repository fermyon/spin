package tests

import (
	"testing"

	"github.com/fermyon/spin/e2e-tests/internal/framework"
	"github.com/fermyon/spin/e2e-tests/internal/spin"
)

func httpCTestcase() framework.Testcase {
	return framework.Testcase{
		Name:     "http-c",
		Template: "http-c",
		AppName:  "http-c-test",
		SubTestsExecutor: func(t *testing.T, metadata *spin.Metadata) {
			assertGetWorks(t, metadata.GetRouteWithName(metadata.AppName).RouteURL, "Hello from WAGI/1\n")
		},
	}
}
