#include <stdlib.h>
#include "wasi-outbound-http.h"

__attribute__((weak, export_name("canonical_abi_realloc"))) void *canonical_abi_realloc(
    void *ptr,
    size_t orig_size,
    size_t org_align,
    size_t new_size)
{
  void *ret = realloc(ptr, new_size);
  if (!ret)
    abort();
  return ret;
}

__attribute__((weak, export_name("canonical_abi_free"))) void canonical_abi_free(
    void *ptr,
    size_t size,
    size_t align)
{
  free(ptr);
}
#include <string.h>

void wasi_outbound_http_string_set(wasi_outbound_http_string_t *ret, const char *s)
{
  ret->ptr = (char *)s;
  ret->len = strlen(s);
}

void wasi_outbound_http_string_dup(wasi_outbound_http_string_t *ret, const char *s)
{
  ret->len = strlen(s);
  ret->ptr = canonical_abi_realloc(NULL, 0, 1, ret->len);
  memcpy(ret->ptr, s, ret->len);
}

void wasi_outbound_http_string_free(wasi_outbound_http_string_t *ret)
{
  canonical_abi_free(ret->ptr, ret->len, 1);
  ret->ptr = NULL;
  ret->len = 0;
}
void wasi_outbound_http_body_free(wasi_outbound_http_body_t *ptr)
{
  canonical_abi_free(ptr->ptr, ptr->len * 1, 1);
}
void wasi_outbound_http_tuple2_string_string_free(wasi_outbound_http_tuple2_string_string_t *ptr)
{
  wasi_outbound_http_string_free(&ptr->f0);
  wasi_outbound_http_string_free(&ptr->f1);
}
void wasi_outbound_http_headers_free(wasi_outbound_http_headers_t *ptr)
{
  for (size_t i = 0; i < ptr->len; i++)
  {
    wasi_outbound_http_tuple2_string_string_free(&ptr->ptr[i]);
  }
  canonical_abi_free(ptr->ptr, ptr->len * 16, 4);
}
void wasi_outbound_http_params_free(wasi_outbound_http_params_t *ptr)
{
  for (size_t i = 0; i < ptr->len; i++)
  {
    wasi_outbound_http_tuple2_string_string_free(&ptr->ptr[i]);
  }
  canonical_abi_free(ptr->ptr, ptr->len * 16, 4);
}
void wasi_outbound_http_uri_free(wasi_outbound_http_uri_t *ptr)
{
  canonical_abi_free(ptr->ptr, ptr->len * 1, 1);
}
void wasi_outbound_http_option_body_free(wasi_outbound_http_option_body_t *ptr)
{
  switch (ptr->tag)
  {
  case 1:
  {
    wasi_outbound_http_body_free(&ptr->val);
    break;
  }
  }
}
void wasi_outbound_http_request_free(wasi_outbound_http_request_t *ptr)
{
  wasi_outbound_http_uri_free(&ptr->uri);
  wasi_outbound_http_headers_free(&ptr->headers);
  wasi_outbound_http_params_free(&ptr->params);
  wasi_outbound_http_option_body_free(&ptr->body);
}
void wasi_outbound_http_option_headers_free(wasi_outbound_http_option_headers_t *ptr)
{
  switch (ptr->tag)
  {
  case 1:
  {
    wasi_outbound_http_headers_free(&ptr->val);
    break;
  }
  }
}
void wasi_outbound_http_response_free(wasi_outbound_http_response_t *ptr)
{
  wasi_outbound_http_option_headers_free(&ptr->headers);
  wasi_outbound_http_option_body_free(&ptr->body);
}
typedef struct
{
  // 0 if `val` is `ok`, 1 otherwise
  uint8_t tag;
  union
  {
    wasi_outbound_http_response_t ok;
    wasi_outbound_http_http_error_t err;
  } val;
} wasi_outbound_http_expected_response_http_error_t;
static int64_t RET_AREA[8];
__attribute__((import_module("wasi-outbound-http"), import_name("request"))) void __wasm_import_wasi_outbound_http_request(int32_t, int32_t, int32_t, int32_t, int32_t, int32_t, int32_t, int32_t, int32_t, int32_t, int32_t);
wasi_outbound_http_http_error_t wasi_outbound_http_request(wasi_outbound_http_request_t *req, wasi_outbound_http_response_t *ret0)
{
  int32_t variant;
  switch ((int32_t)(*req).method)
  {
  case 0:
  {
    variant = 0;
    break;
  }
  case 1:
  {
    variant = 1;
    break;
  }
  case 2:
  {
    variant = 2;
    break;
  }
  case 3:
  {
    variant = 3;
    break;
  }
  case 4:
  {
    variant = 4;
    break;
  }
  case 5:
  {
    variant = 5;
    break;
  }
  case 6:
  {
    variant = 6;
    break;
  }
  }
  int32_t variant8;
  int32_t variant9;
  int32_t variant10;
  switch ((int32_t)((*req).body).tag)
  {
  case 0:
  {
    variant8 = 0;
    variant9 = 0;
    variant10 = 0;
    break;
  }
  case 1:
  {
    const wasi_outbound_http_body_t *payload7 = &((*req).body).val;
    variant8 = 1;
    variant9 = (int32_t)(*payload7).ptr;
    variant10 = (int32_t)(*payload7).len;
    break;
  }
  }
  int32_t ptr = (int32_t)&RET_AREA;
  __wasm_import_wasi_outbound_http_request(variant, (int32_t)((*req).uri).ptr, (int32_t)((*req).uri).len, (int32_t)((*req).headers).ptr, (int32_t)((*req).headers).len, (int32_t)((*req).params).ptr, (int32_t)((*req).params).len, variant8, variant9, variant10, ptr);
  wasi_outbound_http_expected_response_http_error_t variant13;
  variant13.tag = *((int32_t *)(ptr + 0));
  switch ((int32_t)variant13.tag)
  {
  case 0:
  {
    wasi_outbound_http_option_headers_t variant11;
    variant11.tag = *((int32_t *)(ptr + 16));
    switch ((int32_t)variant11.tag)
    {
    case 0:
    {
      break;
    }
    case 1:
    {
      variant11.val = (wasi_outbound_http_headers_t){(wasi_outbound_http_tuple2_string_string_t *)(*((int32_t *)(ptr + 24))), (size_t)(*((int32_t *)(ptr + 32)))};
      break;
    }
    }
    wasi_outbound_http_option_body_t variant12;
    variant12.tag = *((int32_t *)(ptr + 40));
    switch ((int32_t)variant12.tag)
    {
    case 0:
    {
      break;
    }
    case 1:
    {
      variant12.val = (wasi_outbound_http_body_t){(uint8_t *)(*((int32_t *)(ptr + 48))), (size_t)(*((int32_t *)(ptr + 56)))};
      break;
    }
    }
    variant13.val.ok = (wasi_outbound_http_response_t){
        (uint16_t)(*((int32_t *)(ptr + 8))),
        variant11,
        variant12,
    };
    break;
  }
  case 1:
  {
    variant13.val.err = *((int32_t *)(ptr + 8));
    break;
  }
  }
  *ret0 = variant13.val.ok;
  return variant13.tag ? variant13.val.err : -1;
}
