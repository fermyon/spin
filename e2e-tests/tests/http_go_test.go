package tests

import (
	"testing"

	"github.com/fermyon/spin/e2e-tests/internal/framework"
	"github.com/fermyon/spin/e2e-tests/internal/spin"
)

func httpGoTestcase() framework.Testcase {
	return framework.Testcase{
		Name:     "http-go template",
		Template: "http-go",
		AppName:  "http-go-test",
		SubTestsExecutor: func(t *testing.T, metadata *spin.Metadata) {
			assertGetWorks(t, metadata.GetRouteWithName(metadata.AppName).RouteURL, "Hello Fermyon!\n")
		},
	}
}
