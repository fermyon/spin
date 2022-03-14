//nolint:staticcheck

// This file contains the manual conversions between Go HTTP objects
// and Spin HTTP objects, through the auto-generated C bindings for
// the outbound HTTP API.

package http

// #cgo CFLAGS: -Wall
// #include "wasi-outbound-http.h"
// #include<stdlib.h>
import "C"

import (
	"bytes"
	"fmt"
	"io"
	"io/ioutil"
	"net/http"
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
	m, err := method(req.Method)
	if err != nil {
		return nil, err
	}
	spinReq.method = uint8(m)
	spinReq.uri = C.wasi_outbound_http_uri_t{ptr: C.CString(req.URL.String()), len: C.ulong(len(req.URL.String()))}

	hm, _ := toSpinReqHeaders(req.Header)
	spinReq.headers = hm

	b, _ := toSpinReqBody(req.Body)
	spinReq.body = b

	var spinRes C.wasi_outbound_http_response_t

	code := C.wasi_outbound_http_request(&spinReq, &spinRes)

	err = toErr(code, req.URL.String())
	if err != nil {
		return nil, err
	}
	return toStdHttpResp(&spinRes)
}

func method(m string) (int, error) {
	switch m {
	case "GET", "get":
		return 0, nil
	case "POST", "post":
		return 1, nil
	case "PUT", "put":
		return 2, nil
	case "DELETE", "delete":
		return 3, nil
	case "PATCH", "patch":
		return 4, nil
	case "HEAD", "head":
		return 5, nil
	case "OPTIONS", "options":
		return 6, nil
	default:
		return -1, fmt.Errorf("Unknown HTTP method %v", m)
	}
}

// Transform a C outbound HTTP response to a Go *http.Response.
func toStdHttpResp(res *C.wasi_outbound_http_response_t) (*http.Response, error) {
	var body []byte
	if res.body.tag {
		body = make([]byte, res.body.val.len)
		ptr := unsafe.Pointer(res.body.val.ptr)
		p := uintptr(ptr)
		for i := 0; i < len(body); i++ {
			body[i] = byte(*(*C.uint8_t)(unsafe.Pointer(p)))
			p++
		}
	}

	headers, _ := toStdResHeaders(&res.headers)

	t := &http.Response{
		Status:        fmt.Sprintf("%v %v", res.status, http.StatusText(int(res.status))),
		StatusCode:    int(res.status),
		Proto:         "HTTP/1.1",
		ProtoMajor:    1,
		ProtoMinor:    1,
		Body:          ioutil.NopCloser(bytes.NewBuffer(body)),
		ContentLength: int64(len(body)),
		Request:       nil, // we don't really have a request to populate with here
		Header:        headers,
	}
	return t, nil
}

func toSpinReqHeaders(hm http.Header) (C.wasi_outbound_http_headers_t, error) {
	var reqHeaders C.wasi_outbound_http_headers_t
	headersLen := len(hm)

	if headersLen > 0 {
		reqHeaders.len = C.ulong(headersLen)
		var x C.wasi_outbound_http_string_t
		headersPtr := C.malloc(C.size_t(headersLen) * C.size_t(unsafe.Sizeof(x)))
		ptr := (*[1 << 30]C.wasi_outbound_http_tuple2_string_string_t)(unsafe.Pointer(&headersPtr))[:headersLen:headersLen]

		idx := 0
		for k, v := range hm {
			ptr[idx].f0 = C.wasi_outbound_http_string_t{ptr: C.CString(k), len: C.ulong(len(k))}
			ptr[idx].f1 = C.wasi_outbound_http_string_t{ptr: C.CString(v[0]), len: C.ulong(len(v[0]))}
			idx++
		}
		reqHeaders.ptr = &ptr[0]
	}

	return reqHeaders, nil
}

func toSpinReqBody(body io.Reader) (C.wasi_outbound_http_option_body_t, error) {
	var spinBody C.wasi_outbound_http_option_body_t
	spinBody.tag = false

	if body != nil {
		buf := new(bytes.Buffer)
		len, err := buf.ReadFrom(body)
		if err != nil {
			return spinBody, err
		}

		if len > 0 {
			spinBody.tag = true
			var actualBody C.wasi_outbound_http_body_t
			actualBody.len = C.size_t(len)
			actualBody.ptr = &buf.Bytes()[0]
			spinBody.val = actualBody
		}
	}

	return spinBody, nil
}

func toStdResHeaders(hm *C.wasi_outbound_http_option_headers_t) (http.Header, error) {
	if !hm.tag {
		return make(map[string][]string, 0), nil
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

	return headers, nil
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
