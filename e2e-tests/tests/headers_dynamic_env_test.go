package tests

import (
	"net/http"
	"net/url"
	"testing"

	"github.com/fermyon/spin/e2e-tests/internal/framework"
	"github.com/fermyon/spin/e2e-tests/internal/spin"
	"github.com/stretchr/testify/require"
)

func headersDynamicEnvRoutesTestcase() framework.Testcase {
	return framework.Testcase{
		Name:       "headers-dynamic-env-test",
		AppName:    "headers-dynamic-env-test",
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
