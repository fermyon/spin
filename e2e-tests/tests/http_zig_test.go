package tests

import (
	"testing"

	"github.com/fermyon/spin/e2e-tests/internal/framework"
	"github.com/fermyon/spin/e2e-tests/internal/spin"
)

func httpZigTestcase() framework.Testcase {
	return framework.Testcase{
		Name:     "http-zig",
		Template: "http-zig",
		AppName:  "http-zig-test",
		SubTestsExecutor: func(t *testing.T, metadata *spin.Metadata) {
			assertGetWorks(t, metadata.GetRouteWithName(metadata.AppName).RouteURL, "Hello World!\n")
		},
	}
}
