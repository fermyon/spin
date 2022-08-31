#include <stdlib.h>
#include "spin-http.h"

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

void spin_http_string_set(spin_http_string_t *ret, const char *s) {
  ret->ptr = (char*) s;
  ret->len = strlen(s);
}

void spin_http_string_dup(spin_http_string_t *ret, const char *s) {
  ret->len = strlen(s);
  ret->ptr = canonical_abi_realloc(NULL, 0, 1, ret->len);
  memcpy(ret->ptr, s, ret->len);
}

void spin_http_string_free(spin_http_string_t *ret) {
  canonical_abi_free(ret->ptr, ret->len, 1);
  ret->ptr = NULL;
  ret->len = 0;
}
void spin_http_body_free(spin_http_body_t *ptr) {
  canonical_abi_free(ptr->ptr, ptr->len * 1, 1);
}
void spin_http_tuple2_string_string_free(spin_http_tuple2_string_string_t *ptr) {
  spin_http_string_free(&ptr->f0);
  spin_http_string_free(&ptr->f1);
}
void spin_http_headers_free(spin_http_headers_t *ptr) {
  for (size_t i = 0; i < ptr->len; i++) {
    spin_http_tuple2_string_string_free(&ptr->ptr[i]);
  }
  canonical_abi_free(ptr->ptr, ptr->len * 16, 4);
}
void spin_http_params_free(spin_http_params_t *ptr) {
  for (size_t i = 0; i < ptr->len; i++) {
    spin_http_tuple2_string_string_free(&ptr->ptr[i]);
  }
  canonical_abi_free(ptr->ptr, ptr->len * 16, 4);
}
void spin_http_uri_free(spin_http_uri_t *ptr) {
  spin_http_string_free(ptr);
}
void spin_http_option_body_free(spin_http_option_body_t *ptr) {
  if (ptr->is_some) {
    spin_http_body_free(&ptr->val);
  }
}
void spin_http_request_free(spin_http_request_t *ptr) {
  spin_http_uri_free(&ptr->uri);
  spin_http_headers_free(&ptr->headers);
  spin_http_params_free(&ptr->params);
  spin_http_option_body_free(&ptr->body);
}
void spin_http_option_headers_free(spin_http_option_headers_t *ptr) {
  if (ptr->is_some) {
    spin_http_headers_free(&ptr->val);
  }
}
void spin_http_response_free(spin_http_response_t *ptr) {
  spin_http_option_headers_free(&ptr->headers);
  spin_http_option_body_free(&ptr->body);
}

__attribute__((aligned(4)))
static uint8_t RET_AREA[28];
__attribute__((export_name("handle-http-request")))
int32_t __wasm_export_spin_http_handle_http_request(int32_t arg, int32_t arg0, int32_t arg1, int32_t arg2, int32_t arg3, int32_t arg4, int32_t arg5, int32_t arg6, int32_t arg7, int32_t arg8) {
  spin_http_option_body_t option;
  switch (arg6) {
    case 0: {
      option.is_some = false;
      
      break;
    }
    case 1: {
      option.is_some = true;
      
      option.val = (spin_http_body_t) { (uint8_t*)(arg7), (size_t)(arg8) };
      break;
    }
  }spin_http_request_t arg9 = (spin_http_request_t) {
    arg,
    (spin_http_string_t) { (char*)(arg0), (size_t)(arg1) },
    (spin_http_headers_t) { (spin_http_tuple2_string_string_t*)(arg2), (size_t)(arg3) },
    (spin_http_params_t) { (spin_http_tuple2_string_string_t*)(arg4), (size_t)(arg5) },
    option,
  };
  spin_http_response_t ret;
  spin_http_handle_http_request(&arg9, &ret);
  int32_t ptr = (int32_t) &RET_AREA;
  *((int16_t*)(ptr + 0)) = (int32_t) ((ret).status);
  
  if (((ret).headers).is_some) {
    const spin_http_headers_t *payload10 = &((ret).headers).val;
    *((int8_t*)(ptr + 4)) = 1;
    *((int32_t*)(ptr + 12)) = (int32_t) (*payload10).len;
    *((int32_t*)(ptr + 8)) = (int32_t) (*payload10).ptr;
    
  } else {
    *((int8_t*)(ptr + 4)) = 0;
    
  }
  
  if (((ret).body).is_some) {
    const spin_http_body_t *payload12 = &((ret).body).val;
    *((int8_t*)(ptr + 16)) = 1;
    *((int32_t*)(ptr + 24)) = (int32_t) (*payload12).len;
    *((int32_t*)(ptr + 20)) = (int32_t) (*payload12).ptr;
    
  } else {
    *((int8_t*)(ptr + 16)) = 0;
    
  }
  return ptr;
}
