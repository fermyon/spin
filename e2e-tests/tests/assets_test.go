package tests

import (
	"net/http"
	"net/url"
	"testing"

	"github.com/fermyon/spin/e2e-tests/internal/framework"
	"github.com/fermyon/spin/e2e-tests/internal/spin"
	"github.com/stretchr/testify/require"
)

func assetsTestcase() framework.Testcase {
	return framework.Testcase{
		Name:    "assets-test",
		AppName: "assets-test",
		SubTestsExecutor: func(t *testing.T, metadata *spin.Metadata) {
			routeBase, err := url.Parse(metadata.GetRouteWithName("fs").RouteURL)
			require.Nil(t, err, "valid url for app route")

			assertHTTPRequest(t, routeBase.JoinPath("/thisshouldbemounted/1").String(), http.StatusOK, nil, "")
			assertHTTPRequest(t, routeBase.JoinPath("/thisshouldbemounted/2").String(), http.StatusOK, nil, "")
			assertHTTPRequest(t, routeBase.JoinPath("/thisshouldbemounted/3").String(), http.StatusOK, nil, "")
			assertHTTPRequest(t, routeBase.JoinPath("/donotmount/a").String(), http.StatusNotFound, nil, "")
			assertHTTPRequest(t, routeBase.JoinPath("/thisshouldbemounted/thisshouldbeexcluded/4").String(), http.StatusNotFound, nil, "")
		},
	}
}
