package tests

import (
	"net/http"
	"net/url"
	"testing"

	"github.com/fermyon/spin/e2e-tests/internal/framework"
	"github.com/fermyon/spin/e2e-tests/internal/spin"
	"github.com/stretchr/testify/require"
)

func simpleSpinRustTestcase() framework.Testcase {
	return framework.Testcase{
		Name:    "simple-spin-rust-test",
		AppName: "simple-spin-rust-test",
		SubTestsExecutor: func(t *testing.T, metadata *spin.Metadata) {
			routeBase, err := url.Parse(metadata.Base)
			require.Nil(t, err, "valid url for app route")

			assertHTTPRequest(t, routeBase.JoinPath("/test/hello").String(), http.StatusOK, nil, "")
			assertHTTPRequest(t, routeBase.JoinPath("/test/hello/wildcards/should/be/handled").String(), http.StatusOK, nil, "")
			assertHTTPRequest(t, routeBase.JoinPath("/thisshouldfail").String(), http.StatusNotFound, nil, "")
			assertHTTPRequest(t, routeBase.JoinPath("/test/hello/test-placement").String(), http.StatusOK, nil, "")
		},
	}
}
