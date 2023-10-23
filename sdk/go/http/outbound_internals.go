//nolint:staticcheck

// This file contains the manual conversions between Go HTTP objects
// and Spin HTTP objects, through the auto-generated C bindings for
// the outbound HTTP API.

package http

// #cgo CFLAGS: -Wno-unused-parameter -Wno-switch-bool
// #include "wasi-outbound-http.h"
// #include<stdlib.h>
import "C"

import (
	"bytes"
	"fmt"
	"io"
	"io/ioutil"
	"net/http"
	"strings"
	"unsafe"
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
	var spinReq C.wasi_outbound_http_request_t
	var spinRes C.wasi_outbound_http_response_t

	m, err := method(req.Method)
	if err != nil {
		return nil, err
	}
	spinReq.method = uint8(m)
	spinReq.uri = C.wasi_outbound_http_uri_t{
		ptr: C.CString(req.URL.String()),
		len: C.ulong(len(req.URL.String())),
	}
	spinReq.headers = toOutboundHeaders(req.Header)
	spinReq.body, err = toOutboundReqBody(req.Body)
	if err != nil {
		return nil, err
	}

	code := C.wasi_outbound_http_request(&spinReq, &spinRes)

	if err := toErr(code, req.URL.String()); err != nil {
		return nil, err
	}
	return toResponse(&spinRes)
}

func method(m string) (int, error) {
	switch strings.ToUpper(m) {
	case "GET":
		return 0, nil
	case "POST":
		return 1, nil
	case "PUT":
		return 2, nil
	case "DELETE":
		return 3, nil
	case "PATCH":
		return 4, nil
	case "HEAD":
		return 5, nil
	case "OPTIONS":
		return 6, nil
	default:
		return -1, fmt.Errorf("Unknown HTTP method %v", m)
	}
}

// Transform a C outbound HTTP response to a Go *http.Response.
func toResponse(res *C.wasi_outbound_http_response_t) (*http.Response, error) {
	var body []byte
	if res.body.is_some {
		body = C.GoBytes(unsafe.Pointer(res.body.val.ptr), C.int(res.body.val.len))
	}

	t := &http.Response{
		Status:        fmt.Sprintf("%v %v", res.status, http.StatusText(int(res.status))),
		StatusCode:    int(res.status),
		Proto:         "HTTP/1.1",
		ProtoMajor:    1,
		ProtoMinor:    1,
		Body:          ioutil.NopCloser(bytes.NewBuffer(body)),
		ContentLength: int64(len(body)),
		Request:       nil, // we don't really have a request to populate with here
		Header:        toHeaders(&res.headers),
	}
	return t, nil
}

func toOutboundHeaders(hm http.Header) C.wasi_outbound_http_headers_t {
	var reqHeaders C.wasi_outbound_http_headers_t

	headersLen := len(hm)

	if headersLen > 0 {
		reqHeaders.len = C.ulong(headersLen)
		var x C.wasi_outbound_http_tuple2_string_string_t
		reqHeaders.ptr = (*C.wasi_outbound_http_tuple2_string_string_t)(C.malloc(C.size_t(headersLen) * C.size_t(unsafe.Sizeof(x))))
		headers := unsafe.Slice(reqHeaders.ptr, headersLen)

		idx := 0
		for k, v := range hm {
			headers[idx] = newOutboundHeader(k, v[0])
			idx++
		}
	}
	return reqHeaders
}

func toOutboundReqBody(body io.Reader) (C.wasi_outbound_http_option_body_t, error) {
	var spinBody C.wasi_outbound_http_option_body_t
	spinBody.is_some = false

	if body != nil {
		buf := new(bytes.Buffer)
		len, err := buf.ReadFrom(body)
		if err != nil {
			return spinBody, err
		}

		if len > 0 {
			spinBody.is_some = true
			spinBody.val = C.wasi_outbound_http_body_t{
				ptr: &buf.Bytes()[0],
				len: C.size_t(len),
			}
		}
	}

	return spinBody, nil
}

func toHeaders(hm *C.wasi_outbound_http_option_headers_t) http.Header {
	if !hm.is_some {
		return make(map[string][]string, 0)
	}
	headersLen := int(hm.val.len)
	headers := make(http.Header, headersLen)

	var headersArr *C.wasi_outbound_http_tuple2_string_string_t = hm.val.ptr
	headersSlice := unsafe.Slice(headersArr, headersLen)
	for i := 0; i < headersLen; i++ {
		tuple := headersSlice[i]
		k := C.GoStringN(tuple.f0.ptr, C.int(tuple.f0.len))
		v := C.GoStringN(tuple.f1.ptr, C.int(tuple.f1.len))

		headers.Add(k, v)
	}

	return headers
}

func toErr(code C.uint8_t, url string) error {
	switch code {
	case 1:
		return fmt.Errorf("Destination not allowed: %v", url)
	case 2:
		return fmt.Errorf("Invalid URL: %v", url)
	case 3:
		return fmt.Errorf("Error sending request to URL: %v", url)
	case 4:
		return fmt.Errorf("Runtime error")
	default:
		return nil
	}
}

// newOutboundHeader creates a new outboundHeader with the given key/value.
func newOutboundHeader(k, v string) C.wasi_outbound_http_tuple2_string_string_t {
	return C.wasi_outbound_http_tuple2_string_string_t{
		f0: C.wasi_outbound_http_string_t{ptr: C.CString(k), len: C.size_t(len(k))},
		f1: C.wasi_outbound_http_string_t{ptr: C.CString(v), len: C.size_t(len(v))},
	}
}
