#ifndef __BINDINGS_WASI_OUTBOUND_HTTP_H
#define __BINDINGS_WASI_OUTBOUND_HTTP_H
#ifdef __cplusplus
extern "C"
{
  #endif
  
  #include <stdint.h>
  #include <stdbool.h>
  
  typedef struct {
    char *ptr;
    size_t len;
  } wasi_outbound_http_string_t;
  
  void wasi_outbound_http_string_set(wasi_outbound_http_string_t *ret, const char *s);
  void wasi_outbound_http_string_dup(wasi_outbound_http_string_t *ret, const char *s);
  void wasi_outbound_http_string_free(wasi_outbound_http_string_t *ret);
  typedef struct {
    uint8_t *ptr;
    size_t len;
  } wasi_outbound_http_body_t;
  void wasi_outbound_http_body_free(wasi_outbound_http_body_t *ptr);
  typedef struct {
    wasi_outbound_http_string_t f0;
    wasi_outbound_http_string_t f1;
  } wasi_outbound_http_tuple2_string_string_t;
  void wasi_outbound_http_tuple2_string_string_free(wasi_outbound_http_tuple2_string_string_t *ptr);
  typedef struct {
    wasi_outbound_http_tuple2_string_string_t *ptr;
    size_t len;
  } wasi_outbound_http_headers_t;
  void wasi_outbound_http_headers_free(wasi_outbound_http_headers_t *ptr);
  typedef uint8_t wasi_outbound_http_http_error_t;
  #define WASI_OUTBOUND_HTTP_HTTP_ERROR_SUCCESS 0
  #define WASI_OUTBOUND_HTTP_HTTP_ERROR_DESTINATION_NOT_ALLOWED 1
  #define WASI_OUTBOUND_HTTP_HTTP_ERROR_INVALID_URL 2
  #define WASI_OUTBOUND_HTTP_HTTP_ERROR_REQUEST_ERROR 3
  #define WASI_OUTBOUND_HTTP_HTTP_ERROR_RUNTIME_ERROR 4
  #define WASI_OUTBOUND_HTTP_HTTP_ERROR_TOO_MANY_REQUESTS 5
  typedef uint16_t wasi_outbound_http_http_status_t;
  typedef uint8_t wasi_outbound_http_method_t;
  #define WASI_OUTBOUND_HTTP_METHOD_GET 0
  #define WASI_OUTBOUND_HTTP_METHOD_POST 1
  #define WASI_OUTBOUND_HTTP_METHOD_PUT 2
  #define WASI_OUTBOUND_HTTP_METHOD_DELETE 3
  #define WASI_OUTBOUND_HTTP_METHOD_PATCH 4
  #define WASI_OUTBOUND_HTTP_METHOD_HEAD 5
  #define WASI_OUTBOUND_HTTP_METHOD_OPTIONS 6
  typedef struct {
    wasi_outbound_http_tuple2_string_string_t *ptr;
    size_t len;
  } wasi_outbound_http_params_t;
  void wasi_outbound_http_params_free(wasi_outbound_http_params_t *ptr);
  typedef wasi_outbound_http_string_t wasi_outbound_http_uri_t;
  void wasi_outbound_http_uri_free(wasi_outbound_http_uri_t *ptr);
  typedef struct {
    bool is_some;
    wasi_outbound_http_body_t val;
  } wasi_outbound_http_option_body_t;
  void wasi_outbound_http_option_body_free(wasi_outbound_http_option_body_t *ptr);
  typedef struct {
    wasi_outbound_http_method_t method;
    wasi_outbound_http_uri_t uri;
    wasi_outbound_http_headers_t headers;
    wasi_outbound_http_params_t params;
    wasi_outbound_http_option_body_t body;
  } wasi_outbound_http_request_t;
  void wasi_outbound_http_request_free(wasi_outbound_http_request_t *ptr);
  typedef struct {
    bool is_some;
    wasi_outbound_http_headers_t val;
  } wasi_outbound_http_option_headers_t;
  void wasi_outbound_http_option_headers_free(wasi_outbound_http_option_headers_t *ptr);
  typedef struct {
    wasi_outbound_http_http_status_t status;
    wasi_outbound_http_option_headers_t headers;
    wasi_outbound_http_option_body_t body;
  } wasi_outbound_http_response_t;
  void wasi_outbound_http_response_free(wasi_outbound_http_response_t *ptr);
  wasi_outbound_http_http_error_t wasi_outbound_http_request(wasi_outbound_http_request_t *req, wasi_outbound_http_response_t *ret0);
  #ifdef __cplusplus
}
#endif
#endif
