#include <stdlib.h>
#include "wasi-outbound-http.h"

__attribute__((weak, export_name("canonical_abi_realloc")))
void *canonical_abi_realloc(
void *ptr,
size_t orig_size,
size_t org_align,
size_t new_size
) {
  void *ret = realloc(ptr, new_size);
  if (!ret)
  abort();
  return ret;
}

__attribute__((weak, export_name("canonical_abi_free")))
void canonical_abi_free(
void *ptr,
size_t size,
size_t align
) {
  free(ptr);
}
#include <string.h>

void wasi_outbound_http_string_set(wasi_outbound_http_string_t *ret, const char *s) {
  ret->ptr = (char*) s;
  ret->len = strlen(s);
}

void wasi_outbound_http_string_dup(wasi_outbound_http_string_t *ret, const char *s) {
  ret->len = strlen(s);
  ret->ptr = canonical_abi_realloc(NULL, 0, 1, ret->len);
  memcpy(ret->ptr, s, ret->len);
}

void wasi_outbound_http_string_free(wasi_outbound_http_string_t *ret) {
  canonical_abi_free(ret->ptr, ret->len, 1);
  ret->ptr = NULL;
  ret->len = 0;
}
void wasi_outbound_http_body_free(wasi_outbound_http_body_t *ptr) {
  canonical_abi_free(ptr->ptr, ptr->len * 1, 1);
}
void wasi_outbound_http_tuple2_string_string_free(wasi_outbound_http_tuple2_string_string_t *ptr) {
  wasi_outbound_http_string_free(&ptr->f0);
  wasi_outbound_http_string_free(&ptr->f1);
}
void wasi_outbound_http_headers_free(wasi_outbound_http_headers_t *ptr) {
  for (size_t i = 0; i < ptr->len; i++) {
    wasi_outbound_http_tuple2_string_string_free(&ptr->ptr[i]);
  }
  canonical_abi_free(ptr->ptr, ptr->len * 16, 4);
}
void wasi_outbound_http_params_free(wasi_outbound_http_params_t *ptr) {
  for (size_t i = 0; i < ptr->len; i++) {
    wasi_outbound_http_tuple2_string_string_free(&ptr->ptr[i]);
  }
  canonical_abi_free(ptr->ptr, ptr->len * 16, 4);
}
void wasi_outbound_http_uri_free(wasi_outbound_http_uri_t *ptr) {
  wasi_outbound_http_string_free(ptr);
}
void wasi_outbound_http_option_body_free(wasi_outbound_http_option_body_t *ptr) {
  if (ptr->is_some) {
    wasi_outbound_http_body_free(&ptr->val);
  }
}
void wasi_outbound_http_request_free(wasi_outbound_http_request_t *ptr) {
  wasi_outbound_http_uri_free(&ptr->uri);
  wasi_outbound_http_headers_free(&ptr->headers);
  wasi_outbound_http_params_free(&ptr->params);
  wasi_outbound_http_option_body_free(&ptr->body);
}
void wasi_outbound_http_option_headers_free(wasi_outbound_http_option_headers_t *ptr) {
  if (ptr->is_some) {
    wasi_outbound_http_headers_free(&ptr->val);
  }
}
void wasi_outbound_http_response_free(wasi_outbound_http_response_t *ptr) {
  wasi_outbound_http_option_headers_free(&ptr->headers);
  wasi_outbound_http_option_body_free(&ptr->body);
}
typedef struct {
  bool is_err;
  union {
    wasi_outbound_http_response_t ok;
    wasi_outbound_http_http_error_t err;
  } val;
} wasi_outbound_http_expected_response_http_error_t;

__attribute__((aligned(4)))
static uint8_t RET_AREA[32];
__attribute__((import_module("wasi-outbound-http"), import_name("request")))
void __wasm_import_wasi_outbound_http_request(int32_t, int32_t, int32_t, int32_t, int32_t, int32_t, int32_t, int32_t, int32_t, int32_t, int32_t);
wasi_outbound_http_http_error_t wasi_outbound_http_request(wasi_outbound_http_request_t *req, wasi_outbound_http_response_t *ret0) {
  int32_t option;
  int32_t option1;
  int32_t option2;
  
  if (((*req).body).is_some) {
    const wasi_outbound_http_body_t *payload0 = &((*req).body).val;
    option = 1;
    option1 = (int32_t) (*payload0).ptr;
    option2 = (int32_t) (*payload0).len;
    
  } else {
    option = 0;
    option1 = 0;
    option2 = 0;
    
  }
  int32_t ptr = (int32_t) &RET_AREA;
  __wasm_import_wasi_outbound_http_request((int32_t) (*req).method, (int32_t) ((*req).uri).ptr, (int32_t) ((*req).uri).len, (int32_t) ((*req).headers).ptr, (int32_t) ((*req).headers).len, (int32_t) ((*req).params).ptr, (int32_t) ((*req).params).len, option, option1, option2, ptr);
  wasi_outbound_http_expected_response_http_error_t expected;
  switch ((int32_t) (*((uint8_t*) (ptr + 0)))) {
    case 0: {
      expected.is_err = false;
      wasi_outbound_http_option_headers_t option3;
      switch ((int32_t) (*((uint8_t*) (ptr + 8)))) {
        case 0: {
          option3.is_some = false;
          
          break;
        }
        case 1: {
          option3.is_some = true;
          
          option3.val = (wasi_outbound_http_headers_t) { (wasi_outbound_http_tuple2_string_string_t*)(*((int32_t*) (ptr + 12))), (size_t)(*((int32_t*) (ptr + 16))) };
          break;
        }
      }wasi_outbound_http_option_body_t option4;
      switch ((int32_t) (*((uint8_t*) (ptr + 20)))) {
        case 0: {
          option4.is_some = false;
          
          break;
        }
        case 1: {
          option4.is_some = true;
          
          option4.val = (wasi_outbound_http_body_t) { (uint8_t*)(*((int32_t*) (ptr + 24))), (size_t)(*((int32_t*) (ptr + 28))) };
          break;
        }
      }
      expected.val.ok = (wasi_outbound_http_response_t) {
        (uint16_t) ((int32_t) (*((uint16_t*) (ptr + 4)))),
        option3,
        option4,
      };
      break;
    }
    case 1: {
      expected.is_err = true;
      
      expected.val.err = (int32_t) (*((uint8_t*) (ptr + 4)));
      break;
    }
  }*ret0 = expected.val.ok;
  return expected.is_err ? expected.val.err : -1;
}
