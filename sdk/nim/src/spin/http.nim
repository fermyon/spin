import private/internals
import http/private/wit/spin_http
import http/private/[types, utils]

export types

template httpComponent*(handler: proc (req: Request): Response) =
  proc handleHttpRequest(req: ptr spin_http_request_t,
                        res: ptr spin_http_response_t)
                        {.exportc: "spin_http_handle_http_request".} =
    defer: spinHttpRequestFree(req)
    wasm_call_ctors()
    let request = fromSpin(req[])
    let response = handler(request)
    res.status = response.status
    res.headers = response.headers.toSpin
    res.body = response.body.toSpin
