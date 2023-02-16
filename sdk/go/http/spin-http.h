#ifndef __BINDINGS_SPIN_HTTP_H
#define __BINDINGS_SPIN_HTTP_H
#ifdef __cplusplus
extern "C"
{
  #endif
  
  #include <stdint.h>
  #include <stdbool.h>
  
  typedef struct {
    char *ptr;
    size_t len;
  } spin_http_string_t;
  
  void spin_http_string_set(spin_http_string_t *ret, const char *s);
  void spin_http_string_dup(spin_http_string_t *ret, const char *s);
  void spin_http_string_free(spin_http_string_t *ret);
  typedef struct {
    uint8_t *ptr;
    size_t len;
  } spin_http_body_t;
  void spin_http_body_free(spin_http_body_t *ptr);
  typedef struct {
    spin_http_string_t f0;
    spin_http_string_t f1;
  } spin_http_tuple2_string_string_t;
  void spin_http_tuple2_string_string_free(spin_http_tuple2_string_string_t *ptr);
  typedef struct {
    spin_http_tuple2_string_string_t *ptr;
    size_t len;
  } spin_http_headers_t;
  void spin_http_headers_free(spin_http_headers_t *ptr);
  typedef uint8_t spin_http_http_error_t;
  #define SPIN_HTTP_HTTP_ERROR_SUCCESS 0
  #define SPIN_HTTP_HTTP_ERROR_DESTINATION_NOT_ALLOWED 1
  #define SPIN_HTTP_HTTP_ERROR_INVALID_URL 2
  #define SPIN_HTTP_HTTP_ERROR_REQUEST_ERROR 3
  #define SPIN_HTTP_HTTP_ERROR_RUNTIME_ERROR 4
  #define SPIN_HTTP_HTTP_ERROR_TOO_MANY_REQUESTS 5
  typedef uint16_t spin_http_http_status_t;
  typedef uint8_t spin_http_method_t;
  #define SPIN_HTTP_METHOD_GET 0
  #define SPIN_HTTP_METHOD_POST 1
  #define SPIN_HTTP_METHOD_PUT 2
  #define SPIN_HTTP_METHOD_DELETE 3
  #define SPIN_HTTP_METHOD_PATCH 4
  #define SPIN_HTTP_METHOD_HEAD 5
  #define SPIN_HTTP_METHOD_OPTIONS 6
  typedef struct {
    spin_http_tuple2_string_string_t *ptr;
    size_t len;
  } spin_http_params_t;
  void spin_http_params_free(spin_http_params_t *ptr);
  typedef spin_http_string_t spin_http_uri_t;
  void spin_http_uri_free(spin_http_uri_t *ptr);
  typedef struct {
    bool is_some;
    spin_http_body_t val;
  } spin_http_option_body_t;
  void spin_http_option_body_free(spin_http_option_body_t *ptr);
  typedef struct {
    spin_http_method_t method;
    spin_http_uri_t uri;
    spin_http_headers_t headers;
    spin_http_params_t params;
    spin_http_option_body_t body;
  } spin_http_request_t;
  void spin_http_request_free(spin_http_request_t *ptr);
  typedef struct {
    bool is_some;
    spin_http_headers_t val;
  } spin_http_option_headers_t;
  void spin_http_option_headers_free(spin_http_option_headers_t *ptr);
  typedef struct {
    spin_http_http_status_t status;
    spin_http_option_headers_t headers;
    spin_http_option_body_t body;
  } spin_http_response_t;
  void spin_http_response_free(spin_http_response_t *ptr);
  void spin_http_handle_http_request(spin_http_request_t *req, spin_http_response_t *ret0);
  #ifdef __cplusplus
}
#endif
#endif
