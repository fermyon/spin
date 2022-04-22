#include <stdlib.h>
#include <spin-http.h>

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
  canonical_abi_free(ptr->ptr, ptr->len * 1, 1);
}
void spin_http_option_body_free(spin_http_option_body_t *ptr) {
  switch (ptr->tag) {
    case 1: {
      spin_http_body_free(&ptr->val);
      break;
    }
  }
}
void spin_http_request_free(spin_http_request_t *ptr) {
  spin_http_uri_free(&ptr->uri);
  spin_http_headers_free(&ptr->headers);
  spin_http_params_free(&ptr->params);
  spin_http_option_body_free(&ptr->body);
}
void spin_http_option_headers_free(spin_http_option_headers_t *ptr) {
  switch (ptr->tag) {
    case 1: {
      spin_http_headers_free(&ptr->val);
      break;
    }
  }
}
void spin_http_response_free(spin_http_response_t *ptr) {
  spin_http_option_headers_free(&ptr->headers);
  spin_http_option_body_free(&ptr->body);
}
static int64_t RET_AREA[7];
__attribute__((export_name("handle-http-request")))
int32_t __wasm_export_spin_http_handle_http_request(int32_t arg, int32_t arg0, int32_t arg1, int32_t arg2, int32_t arg3, int32_t arg4, int32_t arg5, int32_t arg6, int32_t arg7, int32_t arg8) {
  spin_http_option_body_t variant;
  variant.tag = arg6;
  switch ((int32_t) variant.tag) {
    case 0: {
      break;
    }
    case 1: {
      variant.val = (spin_http_body_t) { (uint8_t*)(arg7), (size_t)(arg8) };
      break;
    }
  }
  spin_http_request_t arg9 = (spin_http_request_t) {
    arg,
    (spin_http_uri_t) { (char*)(arg0), (size_t)(arg1) },
    (spin_http_headers_t) { (spin_http_tuple2_string_string_t*)(arg2), (size_t)(arg3) },
    (spin_http_params_t) { (spin_http_tuple2_string_string_t*)(arg4), (size_t)(arg5) },
    variant,
  };
  spin_http_response_t ret;
  spin_http_handle_http_request(&arg9, &ret);
  int32_t variant11;
  int32_t variant12;
  int32_t variant13;
  switch ((int32_t) ((ret).headers).tag) {
    case 0: {
      variant11 = 0;
      variant12 = 0;
      variant13 = 0;
      break;
    }
    case 1: {
      const spin_http_headers_t *payload10 = &((ret).headers).val;
      variant11 = 1;
      variant12 = (int32_t) (*payload10).ptr;
      variant13 = (int32_t) (*payload10).len;
      break;
    }
  }
  int32_t variant16;
  int32_t variant17;
  int32_t variant18;
  switch ((int32_t) ((ret).body).tag) {
    case 0: {
      variant16 = 0;
      variant17 = 0;
      variant18 = 0;
      break;
    }
    case 1: {
      const spin_http_body_t *payload15 = &((ret).body).val;
      variant16 = 1;
      variant17 = (int32_t) (*payload15).ptr;
      variant18 = (int32_t) (*payload15).len;
      break;
    }
  }
  int32_t ptr = (int32_t) &RET_AREA;
  *((int32_t*)(ptr + 48)) = variant18;
  *((int32_t*)(ptr + 40)) = variant17;
  *((int32_t*)(ptr + 32)) = variant16;
  *((int32_t*)(ptr + 24)) = variant13;
  *((int32_t*)(ptr + 16)) = variant12;
  *((int32_t*)(ptr + 8)) = variant11;
  *((int32_t*)(ptr + 0)) = (int32_t) ((ret).status);
  return ptr;
}
