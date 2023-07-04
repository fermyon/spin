//nolint:staticcheck

// This file contains the manual conversions between Go HTTP objects
// and Spin HTTP objects, through the auto-generated wit-bindgen bindings for
// the outbound HTTP API.

package http

import (
	"bytes"
	"fmt"
	"io"
	"io/ioutil"
	"net/http"

	"github.com/fermyon/spin/sdk/go/generated"
)

func get(url string) (*http.Response, error) {
	req, err := http.NewRequest("GET", url, nil)
	if err != nil {
		return nil, err
	}

	return send(req)
}

func post(url string, contentType string, body io.Reader) (*http.Response, error) {
	req, err := http.NewRequest("POST", url, body)
	if err != nil {
		return nil, err
	}
	req.Header.Set("Content-Type", contentType)

	return send(req)
}

func send(req *http.Request) (*http.Response, error) {
	r := http_trigger.FermyonSpinHttpTypesRequest{
		Method: http_trigger.FermyonSpinHttpTypesMethodGet(),
	}
	res := http_trigger.FermyonSpinHttpSendRequest(r)
	if res.IsErr() {
		return nil, toErr(res.UnwrapErr())
	}
	return toResponse(res.Unwrap())
}

// Transform a C outbound HTTP response to a Go *http.Response.
func toResponse(res http_trigger.FermyonSpinHttpTypesResponse) (*http.Response, error) {
	var body []byte
	if res.Body.IsSome() {
		body = res.Body.Unwrap()
	}

	t := &http.Response{
		Status:        fmt.Sprintf("%v %v", res.Status, http.StatusText(int(res.Status))),
		StatusCode:    int(res.Status),
		Proto:         "HTTP/1.1",
		ProtoMajor:    1,
		ProtoMinor:    1,
		Body:          ioutil.NopCloser(bytes.NewBuffer(body)),
		ContentLength: int64(len(body)),
		Request:       nil, // we don't really have a request to populate with here
		Header:        toHeaders(&res.Headers),
	}
	return t, nil
}

func toHeaders(h *http_trigger.Option[[]http_trigger.FermyonSpinHttpTypesTuple2StringStringT]) http.Header {
	if !h.IsNone() {
		return make(map[string][]string, 0)
	}
	hm := h.Unwrap()
	headersLen := len(hm)
	headers := make(http.Header, headersLen)

	for _, t := range hm {
		headers.Add(t.F0, t.F1)
	}

	return headers
}

func toErr(err http_trigger.FermyonSpinHttpTypesHttpError) error {
	switch err {
	case http_trigger.FermyonSpinHttpTypesHttpErrorRequestError():
		return fmt.Errorf("")
	// TODO: other errors
	default:
		return nil
	}
}
