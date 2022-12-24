package tests

import (
	"net/http"
	"os"
	"testing"

	"github.com/fermyon/spin/e2e-tests/internal/framework"
	"github.com/fermyon/spin/e2e-tests/internal/httputil"
	"github.com/fermyon/spin/e2e-tests/internal/spin"
	"github.com/stretchr/testify/require"
)

func TestSpinTemplatesUsingSpinUp(t *testing.T) {
	controller := spin.WithSpinUp()
	testSpinTemplates(t, controller)
}

// func TestSpinTemplatesUsingCloud(t *testing.T) {
// 	withcloud := spin.WithFermyonCloud()

// 	err := withcloud.Login()
// 	require.NoError(t, err)

// 	testSpinTemplates(t, withcloud)
// }

func testSpinTemplates(t *testing.T, controller spin.Controller) {
	tmpPluginsDir, err := os.MkdirTemp("", "spin-plugins-tmpdir")
	defer os.RemoveAll(tmpPluginsDir)

	require.Nil(t, err)
	os.Setenv("TEST_PLUGINS_DIRECTORY", tmpPluginsDir)

	err = controller.TemplatesInstall("--git", "https://github.com/fermyon/spin")
	require.NoError(t, err)

	for _, testcase := range []framework.Testcase{
		httpGoTestcase(),
		httpRustTestcase(),
		httpCTestcase(),
		httpZigTestcase(),
		httpGrainTestcase(),
		httpTSTestcase(),
		httpJSTestcase(),
		assetsTestcase(),
		simpleSpinRustTestcase(),
		headersEnvRoutesTestcase(),
		headersDynamicEnvRoutesTestcase(),
	} {
		func(testcase framework.Testcase, t *testing.T) {
			t.Run(testcase.Name, func(t *testing.T) {
				t.Parallel()
				testcase.Run(t, controller)
			})
		}(testcase, t)
	}
}

func assertGetWorks(t *testing.T, approute, expectedBody string) {
	assertHTTPRequest(t, approute, http.StatusOK, nil, expectedBody)
}

func assertHTTPRequest(t *testing.T, approute string, expectedCode int, expectedHeaders map[string]string, expectedBody string) {
	resp, err := httputil.Get(approute)
	require.NoError(t, err)
	require.NotNil(t, resp)

	require.Equal(t, expectedCode, resp.StatusCode, "http status code for url %q", approute)

	if len(expectedHeaders) > 0 {
		for k, v := range expectedHeaders {
			require.Equal(t, v, resp.Header.Get(k), "http header %q in resp for url %q", k, approute)
		}
	}

	if expectedBody != "" {
		body, err := httputil.BodyString(resp)
		require.NoError(t, err)
		require.Equal(t, expectedBody, body, "http response body for url %s", approute)
	}
}
