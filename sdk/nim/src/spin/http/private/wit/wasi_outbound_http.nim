# Generated with nimterop v0.6.13 and edited manually

from os import splitPath

{.push hint[ConvFromXtoItselfNotNeeded]: off.}

{.compile: currentSourcePath().splitPath.head & "/c/spin-http.c".}

{.pragma: impwasioutboundhttpHdr, header: currentSourcePath().splitPath.head & "/c/wasi-outbound-http.h".}
{.experimental: "codeReordering".}
const
  WASI_OUTBOUND_HTTP_HTTP_ERROR_SUCCESS* = 0
  WASI_OUTBOUND_HTTP_HTTP_ERROR_DESTINATION_NOT_ALLOWED* = 1
  WASI_OUTBOUND_HTTP_HTTP_ERROR_INVALID_URL* = 2
  WASI_OUTBOUND_HTTP_HTTP_ERROR_REQUEST_ERROR* = 3
  WASI_OUTBOUND_HTTP_HTTP_ERROR_RUNTIME_ERROR* = 4
  WASI_OUTBOUND_HTTP_METHOD_GET* = 0
  WASI_OUTBOUND_HTTP_METHOD_POST* = 1
  WASI_OUTBOUND_HTTP_METHOD_PUT* = 2
  WASI_OUTBOUND_HTTP_METHOD_DELETE* = 3
  WASI_OUTBOUND_HTTP_METHOD_PATCH* = 4
  WASI_OUTBOUND_HTTP_METHOD_HEAD* = 5
  WASI_OUTBOUND_HTTP_METHOD_OPTIONS* = 6
type
  wasi_outbound_http_string_t* {.bycopy, importc, impwasioutboundhttpHdr.} = object
    `ptr`*: cstring
    len*: uint

  wasi_outbound_http_body_t* {.bycopy, importc, impwasioutboundhttpHdr.} = object
    `ptr`*: ptr uint8
    len*: uint

  wasi_outbound_http_tuple2_string_string_t* {.bycopy, importc,
      impwasioutboundhttpHdr.} = object
    f0*: wasi_outbound_http_string_t
    f1*: wasi_outbound_http_string_t

  wasi_outbound_http_headers_t* {.bycopy, importc, impwasioutboundhttpHdr.} = object
    `ptr`*: ptr wasi_outbound_http_tuple2_string_string_t
    len*: uint

  wasi_outbound_http_http_error_t* {.importc, impwasioutboundhttpHdr.} = uint8
  wasi_outbound_http_http_status_t* {.importc, impwasioutboundhttpHdr.} = uint16
  wasi_outbound_http_method_t* {.importc, impwasioutboundhttpHdr.} = uint8
  wasi_outbound_http_params_t* {.bycopy, importc, impwasioutboundhttpHdr.} = object
    `ptr`*: ptr wasi_outbound_http_tuple2_string_string_t
    len*: uint

  wasi_outbound_http_uri_t* {.importc, impwasioutboundhttpHdr.} = wasi_outbound_http_string_t
  wasi_outbound_http_option_body_t* {.bycopy, importc, impwasioutboundhttpHdr.} = object
    is_some*: bool
    val*: wasi_outbound_http_body_t

  wasi_outbound_http_request_t* {.bycopy, importc, impwasioutboundhttpHdr.} = object
    `method`*: wasi_outbound_http_method_t
    uri*: wasi_outbound_http_uri_t
    headers*: wasi_outbound_http_headers_t
    params*: wasi_outbound_http_params_t
    body*: wasi_outbound_http_option_body_t

  wasi_outbound_http_option_headers_t* {.bycopy, importc, impwasioutboundhttpHdr.} = object
    is_some*: bool
    val*: wasi_outbound_http_headers_t

  wasi_outbound_http_response_t* {.bycopy, importc, impwasioutboundhttpHdr.} = object
    status*: wasi_outbound_http_http_status_t
    headers*: wasi_outbound_http_option_headers_t
    body*: wasi_outbound_http_option_body_t

proc wasi_outbound_http_string_set*(ret: ptr wasi_outbound_http_string_t;
                                    s: cstring) {.importc, cdecl,
    impwasioutboundhttpHdr.}
proc wasi_outbound_http_string_dup*(ret: ptr wasi_outbound_http_string_t;
                                    s: cstring) {.importc, cdecl,
    impwasioutboundhttpHdr.}
proc wasi_outbound_http_string_free*(ret: ptr wasi_outbound_http_string_t) {.
    importc, cdecl, impwasioutboundhttpHdr.}
proc wasi_outbound_http_body_free*(`ptr`: ptr wasi_outbound_http_body_t) {.
    importc, cdecl, impwasioutboundhttpHdr.}
proc wasi_outbound_http_tuple2_string_string_free*(
    `ptr`: ptr wasi_outbound_http_tuple2_string_string_t) {.importc, cdecl,
    impwasioutboundhttpHdr.}
proc wasi_outbound_http_headers_free*(`ptr`: ptr wasi_outbound_http_headers_t) {.
    importc, cdecl, impwasioutboundhttpHdr.}
proc wasi_outbound_http_params_free*(`ptr`: ptr wasi_outbound_http_params_t) {.
    importc, cdecl, impwasioutboundhttpHdr.}
proc wasi_outbound_http_uri_free*(`ptr`: ptr wasi_outbound_http_uri_t) {.
    importc, cdecl, impwasioutboundhttpHdr.}
proc wasi_outbound_http_option_body_free*(
    `ptr`: ptr wasi_outbound_http_option_body_t) {.importc, cdecl,
    impwasioutboundhttpHdr.}
proc wasi_outbound_http_request_free*(`ptr`: ptr wasi_outbound_http_request_t) {.
    importc, cdecl, impwasioutboundhttpHdr.}
proc wasi_outbound_http_option_headers_free*(
    `ptr`: ptr wasi_outbound_http_option_headers_t) {.importc, cdecl,
    impwasioutboundhttpHdr.}
proc wasi_outbound_http_response_free*(`ptr`: ptr wasi_outbound_http_response_t) {.
    importc, cdecl, impwasioutboundhttpHdr.}
proc wasi_outbound_http_request*(req: ptr wasi_outbound_http_request_t;
                                 ret0: ptr wasi_outbound_http_response_t): wasi_outbound_http_http_error_t {.
    importc, cdecl, impwasioutboundhttpHdr.}
{.pop.}
