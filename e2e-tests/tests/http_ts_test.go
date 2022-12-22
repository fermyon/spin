package tests

import (
	"os/exec"
	"testing"

	"github.com/fermyon/spin/e2e-tests/internal/framework"
	"github.com/fermyon/spin/e2e-tests/internal/spin"
)

func httpTSTestcase() framework.Testcase {
	return framework.Testcase{
		Name:                "http-ts",
		AppName:             "http-ts-test",
		Plugins:             []string{"js2wasm"},
		Template:            "http-ts",
		TemplateInstallArgs: []string{"--git", "https://github.com/fermyon/spin-js-sdk", "--update"},
		PreBuildHooks: []*exec.Cmd{
			exec.Command("npm", "install"),
		},
		SubTestsExecutor: func(t *testing.T, metadata *spin.Metadata) {
			assertGetWorks(t, metadata.GetRouteWithName(metadata.AppName).RouteURL, "Hello from TS-SDK")
		},
	}
}
