// Package http contains the helper functions for writing Spin HTTP components
// in TinyGo, as well as for sending outbound HTTP requests.
package http

import (
	"fmt"
	"io"
	"net/http"
	"os"

	"github.com/julienschmidt/httprouter"
)

const (
	// The application base path.
	HeaderBasePath = "spin-base-path"
	// The component route pattern matched, _excluding_ any wildcard indicator.
	HeaderComponentRoot = "spin-component-route"
	// The full URL of the request. This includes full host and scheme information.
	HeaderFullUrl = "spin-full-url"
	// The part of the request path that was matched by the route (including
	// the base and wildcard indicator if present).
	HeaderMatchedRoute = "spin-matched-route"
	// The request path relative to the component route (including any base).
	HeaderPathInfo = "spin-path-info"
	// The component route pattern matched, as written in the component
	// manifest (that is, _excluding_ the base, but including the wildcard
	// indicator if present).
	HeaderRawComponentRoot = "spin-raw-component-route"
	// The client address for the request.
	HeaderClientAddr = "spin-client-addr"
)

// Override the default HTTP client to be compatible with the Spin SDK.
func init() {
	http.DefaultClient = NewClient()
}

// Router is a http.Handler which can be used to dispatch requests to different
// handler functions via configurable routes
type Router = httprouter.Router

// Params is a Param-slice, as returned by the router.
// The slice is ordered, the first URL parameter is also the first slice value.
// It is therefore safe to read values by the index.
type Params = httprouter.Params

// Param is a single URL parameter, consisting of a key and a value.
type Param = httprouter.Param

// RouterHandle is a function that can be registered to a route to handle HTTP
// requests. Like http.HandlerFunc, but has a third parameter for the values of
// wildcards (variables).
type RouterHandle = httprouter.Handle

// New returns a new initialized Router.
// Path auto-correction, including trailing slashes, is enabled by default.
func NewRouter() *Router {
	return httprouter.New()
}

// NewTransport returns http.RoundTripper backed by Spin SDK
func NewTransport() http.RoundTripper {
	return &Transport{}
}

// Transport implements http.RoundTripper
type Transport struct{}

// RoundTrip makes roundtrip using Spin SDK
func (r *Transport) RoundTrip(req *http.Request) (*http.Response, error) {
	return Send(req)
}

// NewClient returns a new HTTP client compatible with the Spin SDK
func NewClient() *http.Client {
	return &http.Client{
		Transport: &Transport{},
	}
}

// handler is the function that will be called by the http trigger in Spin.
var handler = defaultHandler

// defaultHandler is a placeholder for returning a useful error to stderr when
// the handler is not set.
var defaultHandler = func(http.ResponseWriter, *http.Request) {
	fmt.Fprintln(os.Stderr, "http handler undefined")
}

// Handle sets the handler function for the http trigger.
// It must be set in an init() function.
func Handle(fn func(http.ResponseWriter, *http.Request)) {
	handler = fn
}

// Get creates a GET HTTP request to a given URL and returns the HTTP response.
// The destination of the request must be explicitly allowed in the Spin application
// configuration, otherwise the request will not be sent.
func Get(url string) (*http.Response, error) {
	return get(url)
}

// Post creates a POST HTTP request and returns the HTTP response.
// The destination of the request must be explicitly allowed in the Spin application
// configuration, otherwise the request will not be sent.
func Post(url string, contentType string, body io.Reader) (*http.Response, error) {
	return post(url, contentType, body)
}

// Send sends an HTTP request and return the HTTP response.
// The destination of the request must be explicitly allowed in the Spin application
// configuration, otherwise the request will not be sent.
func Send(req *http.Request) (*http.Response, error) {
	return send(req)
}
