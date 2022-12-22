package tests

import (
	"os/exec"
	"testing"

	"github.com/fermyon/spin/e2e-tests/internal/framework"
	"github.com/fermyon/spin/e2e-tests/internal/spin"
)

func httpJSTestcase() framework.Testcase {
	return framework.Testcase{
		Name:                "http-js",
		AppName:             "http-js-test",
		Plugins:             []string{"js2wasm"},
		Template:            "http-js",
		TemplateInstallArgs: []string{"--git", "https://github.com/fermyon/spin-js-sdk", "--update"},
		PreBuildHooks: []*exec.Cmd{
			exec.Command("npm", "install"),
		},
		SubTestsExecutor: func(t *testing.T, metadata *spin.Metadata) {
			assertGetWorks(t, metadata.GetRouteWithName(metadata.AppName).RouteURL, "Hello from JS-SDK")
		},
	}
}
