// Package http contains the helper functions for writing Spin HTTP components
// in TinyGo, as well as for sending outbound HTTP requests.
package http

import (
	"fmt"
	"io"
	"net/http"
	"os"
)

// handler is the function that will be called by the http trigger in Spin.
var handler = defaultHandler

// defaultHandler is a placeholder for returning a useful error to stdout when
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
