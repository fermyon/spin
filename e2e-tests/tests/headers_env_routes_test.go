package tests

import (
	"net/http"
	"net/url"
	"testing"

	"github.com/fermyon/spin/e2e-tests/internal/framework"
	"github.com/fermyon/spin/e2e-tests/internal/spin"
	"github.com/stretchr/testify/require"
)

func headersEnvRoutesTestcase() framework.Testcase {
	return framework.Testcase{
		Name:    "headers-env-routes-test",
		AppName: "headers-env-routes-test",
		SubTestsExecutor: func(t *testing.T, metadata *spin.Metadata) {
			routeBase, err := url.Parse(metadata.Base)
			require.Nil(t, err, "valid url for app route")

			assertHTTPRequest(t, routeBase.JoinPath("/env").String(), http.StatusOK, nil, "")
			assertHTTPRequest(t, routeBase.JoinPath("/env/foo").String(), http.StatusOK, map[string]string{"env_some_key": "some_value"}, "")
		},
	}
}
