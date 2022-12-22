package tests

import (
	"testing"

	"github.com/fermyon/spin/e2e-tests/internal/framework"
	"github.com/fermyon/spin/e2e-tests/internal/spin"
)

func httpGrainTestcase() framework.Testcase {
	return framework.Testcase{
		Name:     "http-grain",
		Template: "http-grain",
		AppName:  "http-grain-test",
		SubTestsExecutor: func(t *testing.T, metadata *spin.Metadata) {
			assertGetWorks(t, metadata.GetRouteWithName(metadata.AppName).RouteURL, "Hello, World\n")
		},
	}
}
