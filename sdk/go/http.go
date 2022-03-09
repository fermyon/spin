package spin_http

import (
	"net/http"
	"net/http/cgi"
)

// The entrypoint handler for a Spin HTTP component.
//
// This is currently handled using CGI to form the request and reponse,
// but as Go implements support for the component model, the underlying
// implementation of this function can change, but the exported signature
// can continue to always be
// `func Handler(h func(w http.ResponseWriter, r *http.Request)) error`.
func Handler(h func(w http.ResponseWriter, r *http.Request)) error {
	return cgi.Serve(http.HandlerFunc(h))
}
