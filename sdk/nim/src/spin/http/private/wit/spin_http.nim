# Generated with nimterop v0.6.13 and edited manually

from os import splitPath

{.push hint[ConvFromXtoItselfNotNeeded]: off.}

{.compile: currentSourcePath().splitPath.head & "/c/spin-http.c".}

{.pragma: impspinhttpHdr, header: currentSourcePath().splitPath.head & "/c/spin-http.h".}
{.experimental: "codeReordering".}
const
  SPIN_HTTP_HTTP_ERROR_SUCCESS*: spin_http_http_error_t = 0
  SPIN_HTTP_HTTP_ERROR_DESTINATION_NOT_ALLOWED*: spin_http_http_error_t = 1
  SPIN_HTTP_HTTP_ERROR_INVALID_URL*: spin_http_http_error_t = 2
  SPIN_HTTP_HTTP_ERROR_REQUEST_ERROR*: spin_http_http_error_t = 3
  SPIN_HTTP_HTTP_ERROR_RUNTIME_ERROR*: spin_http_http_error_t = 4
  SPIN_HTTP_METHOD_GET*: spin_http_method_t = 0
  SPIN_HTTP_METHOD_POST*: spin_http_method_t = 1
  SPIN_HTTP_METHOD_PUT*: spin_http_method_t = 2
  SPIN_HTTP_METHOD_DELETE*: spin_http_method_t = 3
  SPIN_HTTP_METHOD_PATCH*: spin_http_method_t = 4
  SPIN_HTTP_METHOD_HEAD*: spin_http_method_t = 5
  SPIN_HTTP_METHOD_OPTIONS*: spin_http_method_t = 6
type
  spin_http_string_t* {.bycopy, importc, impspinhttpHdr.} = object
    `ptr`*: cstring
    len*: uint

  spin_http_body_t* {.bycopy, importc, impspinhttpHdr.} = object
    `ptr`*: ptr uint8
    len*: uint

  spin_http_tuple2_string_string_t* {.bycopy, importc, impspinhttpHdr.} = object
    f0*: spin_http_string_t
    f1*: spin_http_string_t

  spin_http_headers_t* {.bycopy, importc, impspinhttpHdr.} = object
    `ptr`*: ptr spin_http_tuple2_string_string_t
    len*: uint

  spin_http_http_error_t* {.importc, impspinhttpHdr.} = uint8
  spin_http_http_status_t* {.importc, impspinhttpHdr.} = uint16
  spin_http_method_t* {.importc, impspinhttpHdr.} = uint8
  spin_http_params_t* {.bycopy, importc, impspinhttpHdr.} = object
    `ptr`*: ptr spin_http_tuple2_string_string_t
    len*: uint

  spin_http_uri_t* {.importc, impspinhttpHdr.} = spin_http_string_t
  spin_http_option_body_t* {.bycopy, importc, impspinhttpHdr.} = object
    is_some*: bool
    val*: spin_http_body_t

  spin_http_request_t* {.bycopy, importc, impspinhttpHdr.} = object
    `method`*: spin_http_method_t
    uri*: spin_http_uri_t
    headers*: spin_http_headers_t
    params*: spin_http_params_t
    body*: spin_http_option_body_t

  spin_http_option_headers_t* {.bycopy, importc, impspinhttpHdr.} = object
    is_some*: bool
    val*: spin_http_headers_t

  spin_http_response_t* {.bycopy, importc, impspinhttpHdr.} = object
    status*: spin_http_http_status_t
    headers*: spin_http_option_headers_t
    body*: spin_http_option_body_t

proc spin_http_string_set*(ret: ptr spin_http_string_t; s: cstring) {.importc,
    cdecl, impspinhttpHdr.}
proc spin_http_string_dup*(ret: ptr spin_http_string_t; s: cstring) {.importc,
    cdecl, impspinhttpHdr.}
proc spin_http_string_free*(ret: ptr spin_http_string_t) {.importc, cdecl,
    impspinhttpHdr.}
proc spin_http_body_free*(`ptr`: ptr spin_http_body_t) {.importc, cdecl,
    impspinhttpHdr.}
proc spin_http_tuple2_string_string_free*(
    `ptr`: ptr spin_http_tuple2_string_string_t) {.importc, cdecl,
    impspinhttpHdr.}
proc spin_http_headers_free*(`ptr`: ptr spin_http_headers_t) {.importc, cdecl,
    impspinhttpHdr.}
proc spin_http_params_free*(`ptr`: ptr spin_http_params_t) {.importc, cdecl,
    impspinhttpHdr.}
proc spin_http_uri_free*(`ptr`: ptr spin_http_uri_t) {.importc, cdecl,
    impspinhttpHdr.}
proc spin_http_option_body_free*(`ptr`: ptr spin_http_option_body_t) {.importc,
    cdecl, impspinhttpHdr.}
proc spin_http_request_free*(`ptr`: ptr spin_http_request_t) {.importc, cdecl,
    impspinhttpHdr.}
proc spin_http_option_headers_free*(`ptr`: ptr spin_http_option_headers_t) {.
    importc, cdecl, impspinhttpHdr.}
proc spin_http_response_free*(`ptr`: ptr spin_http_response_t) {.importc, cdecl,
    impspinhttpHdr.}
proc spin_http_handle_http_request*(req: ptr spin_http_request_t;
                                    ret0: ptr spin_http_response_t) {.importc,
    cdecl, impspinhttpHdr.}
{.pop.}
