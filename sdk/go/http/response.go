package http

import (
	"bytes"
	"net/http"
)

var _ http.ResponseWriter = (*response)(nil)

// response implements http.ResponseWriter
type response struct {
	// status code passed to WriteHeader
	status int

	header http.Header
	w      *bytes.Buffer
}

func newResponse() *response {
	return &response{
		// set default status to StatusOK
		status: http.StatusOK,

		header: make(http.Header),
		w:      new(bytes.Buffer),
	}
}

func (r *response) Header() http.Header {
	return r.header
}

func (r *response) WriteHeader(statusCode int) {
	r.status = statusCode
}

func (r *response) Write(data []byte) (int, error) {
	return r.w.Write(data)
}
