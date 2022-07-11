package http

// #cgo CFLAGS: -Wno-unused-parameter -Wno-switch-bool
// #include<spin-http.h>
// #include<stdlib.h>
import "C"
import (
	"bytes"
	"fmt"
	"io"
	"net/http"
	"os"
	"unsafe"
)

//export spin_http_handle_http_request
func handle_http_request(req *C.spin_http_request_t, res *C.spin_http_response_t) {
	defer C.spin_http_request_free(req)

	var body []byte
	if req.body.is_some {
		body = C.GoBytes(unsafe.Pointer(req.body.val.ptr), C.int(req.body.val.len))
	}
	method := methods[req.method]
	uri := C.GoStringN(req.uri.ptr, C.int(req.uri.len))

	// NOTE Host is not included in the URL
	r, err := http.NewRequest(method, uri, bytes.NewReader(body))
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		res.status = C.uint16_t(http.StatusInternalServerError)
		return
	}

	r.Header = fromSpinHeaders(&req.headers)
	r.Host = r.Header.Get("Host")

	w := newResponse()

	// call user function
	handler(w, r)

	res.status = C.uint16_t(w.status)
	if len(w.header) > 0 {
		res.headers = C.spin_http_option_headers_t{
			is_some: true,
			val:     toSpinHeaders(w.header),
		}
	} else {
		res.headers = C.spin_http_option_headers_t{is_some: false}
	}

	res.body, err = toSpinBody(w.w)
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
	}
}

func toSpinHeaders(hm http.Header) C.spin_http_headers_t {
	var reqHeaders C.spin_http_headers_t

	headersLen := len(hm)

	if headersLen > 0 {
		reqHeaders.len = C.ulong(headersLen)
		var x C.spin_http_string_t
		headersPtr := C.malloc(C.size_t(headersLen) * C.size_t(unsafe.Sizeof(x)))
		ptr := (*[1 << 30]C.spin_http_tuple2_string_string_t)(unsafe.Pointer(&headersPtr))[:headersLen:headersLen]

		idx := 0
		for k, v := range hm {
			ptr[idx] = newSpinHeader(k, v[0])
			idx++
		}

		reqHeaders.ptr = &ptr[0]
	}
	return reqHeaders
}

func toSpinBody(body io.Reader) (C.spin_http_option_body_t, error) {
	var spinBody C.spin_http_option_body_t
	spinBody.is_some = false

	if body == nil {
		return spinBody, nil
	}

	buf := new(bytes.Buffer)
	len, err := buf.ReadFrom(body)
	if err != nil {
		return spinBody, err
	}

	if len > 0 {
		spinBody.is_some = true
		spinBody.val = C.spin_http_body_t{
			ptr: &buf.Bytes()[0],
			len: C.size_t(len),
		}
	}

	return spinBody, nil
}

// newSpinHeader creates a new spinHeader with the given key/value.
func newSpinHeader(k, v string) C.spin_http_tuple2_string_string_t {
	return C.spin_http_tuple2_string_string_t{
		f0: C.spin_http_string_t{ptr: C.CString(k), len: C.size_t(len(k))},
		f1: C.spin_http_string_t{ptr: C.CString(v), len: C.size_t(len(v))},
	}
}

var methods = [...]string{
	"GET",
	"POST",
	"PUT",
	"DELETE",
	"PATCH",
	"HEAD",
	"OPTIONS",
}

func fromSpinHeaders(hm *C.spin_http_headers_t) http.Header {
	headersLen := int(hm.len)
	headers := make(http.Header, headersLen)

	var headersArr *C.spin_http_tuple2_string_string_t = hm.ptr
	headersSlice := unsafe.Slice(headersArr, headersLen)
	for i := 0; i < headersLen; i++ {
		tuple := headersSlice[i]
		k := C.GoStringN(tuple.f0.ptr, C.int(tuple.f0.len))
		v := C.GoStringN(tuple.f1.ptr, C.int(tuple.f1.len))

		headers.Add(k, v)
	}

	return headers
}
