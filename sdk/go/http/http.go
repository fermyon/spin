//nolint:staticcheck

// Package http contains the helper functions for writing Spin HTTP components
// in TinyGo, as well as for sending outbound HTTP requests.
package http

import (
	"io"
	"net/http"
	"net/http/cgi"
)

// HandleRequest is the entrypoint handler for a Spin HTTP component.
//
// This is currently handled using CGI to form the request and reponse,
// but as Go implements support for the component model, the underlying
// implementation of this function can change, but the exported signature
// can continue to always be
// `func HandleRequest(h func(w http.ResponseWriter, r *http.Request)) error`.
func HandleRequest(h func(w http.ResponseWriter, r *http.Request)) error {
	return cgi.Serve(http.HandlerFunc(h))
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
