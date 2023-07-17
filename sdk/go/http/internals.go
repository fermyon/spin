package http

import (
	"bytes"
	"fmt"
	"io"
	"io/ioutil"
	"net/http"
	"os"

	http_trigger "github.com/fermyon/spin/sdk/go/generated"
)

func SetHandler() {
	http_trigger.SetExportsFermyonSpinInboundHttp(Handler{})
}

type Handler struct{}

func (Handler) HandleRequest(req http_trigger.FermyonSpinHttpTypesRequest) http_trigger.FermyonSpinHttpTypesResponse {
	var resp http_trigger.FermyonSpinHttpTypesResponse
	var body []byte
	if req.Body.IsSome() {
		body = req.Body.Unwrap()
	}
	req.Method.Kind()
	method := methods[req.Method.Kind()]
	header := fromSpinHeaders(req.Headers)
	url := header.Get(HeaderFullUrl)

	r, err := http.NewRequest(method, url, bytes.NewReader(body))
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
		return resp
	}

	r.Header = header
	r.Host = r.Header.Get("Host")
	r.RequestURI = req.Uri
	r.RemoteAddr = r.Header.Get(HeaderClientAddr)

	w := newResponse()

	// call user function
	handler(w, r)

	resp.Status = uint16(w.status)
	headers := http_trigger.Option[[]http_trigger.FermyonSpinInboundHttpTuple2StringStringT]{}
	headers.Set(toSpinHeaders(w.Header()))
	resp.Headers = headers

	resp.Body, err = toSpinBody(w.w)
	if err != nil {
		fmt.Fprintln(os.Stderr, err)
	}
	return resp
}

func toSpinHeaders(hm http.Header) []http_trigger.FermyonSpinInboundHttpTuple2StringStringT {
	result := make([]http_trigger.FermyonSpinInboundHttpTuple2StringStringT, len(hm))
	idx := 0
	for k, v := range hm {
		result[idx] = http_trigger.FermyonSpinInboundHttpTuple2StringStringT{
			F0: k,
			F1: v[0],
		}
		idx++
	}
	return result
}

func toSpinBody(body io.Reader) (http_trigger.Option[[]uint8], error) {
	result := http_trigger.Option[[]uint8]{}
	b, err := ioutil.ReadAll(body)
	if err != nil {
		return result, err
	}
	result.Set(b)
	return result, nil
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

func fromSpinHeaders(hm []http_trigger.FermyonSpinInboundHttpTuple2StringStringT) http.Header {
	headers := make(http.Header, len(hm))

	for _, pair := range hm {
		k := pair.F0
		v := pair.F1

		headers.Add(k, v)
	}

	return headers
}
