package tests

import (
	"testing"

	"github.com/fermyon/spin/e2e-tests/internal/framework"
	"github.com/fermyon/spin/e2e-tests/internal/spin"
)

func httpRustTestcase() framework.Testcase {
	return framework.Testcase{
		Name:     "http-rust",
		Template: "http-rust",
		AppName:  "http-rust-test",
		SubTestsExecutor: func(t *testing.T, metadata *spin.Metadata) {
			assertGetWorks(t, metadata.GetRouteWithName(metadata.AppName).RouteURL, "Hello, Fermyon")
		},
	}
}
