#include <stdlib.h>
#include <outbound-redis.h>

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

void outbound_redis_string_set(outbound_redis_string_t *ret, const char *s) {
  ret->ptr = (char*) s;
  ret->len = strlen(s);
}

void outbound_redis_string_dup(outbound_redis_string_t *ret, const char *s) {
  ret->len = strlen(s);
  ret->ptr = canonical_abi_realloc(NULL, 0, 1, ret->len);
  memcpy(ret->ptr, s, ret->len);
}

void outbound_redis_string_free(outbound_redis_string_t *ret) {
  canonical_abi_free(ret->ptr, ret->len, 1);
  ret->ptr = NULL;
  ret->len = 0;
}
void outbound_redis_payload_free(outbound_redis_payload_t *ptr) {
  canonical_abi_free(ptr->ptr, ptr->len * 1, 1);
}
typedef struct {
  // 0 if `val` is `ok`, 1 otherwise
  uint8_t tag;
  union {
    outbound_redis_error_t err;
  } val;
} outbound_redis_expected_void_error_t;
typedef struct {
  // 0 if `val` is `ok`, 1 otherwise
  uint8_t tag;
  union {
    outbound_redis_payload_t ok;
    outbound_redis_error_t err;
  } val;
} outbound_redis_expected_payload_error_t;
static int64_t RET_AREA[3];
__attribute__((import_module("outbound-redis"), import_name("publish")))
void __wasm_import_outbound_redis_publish(int32_t, int32_t, int32_t, int32_t, int32_t, int32_t, int32_t);
outbound_redis_error_t outbound_redis_publish(outbound_redis_string_t *address, outbound_redis_string_t *channel, outbound_redis_payload_t *payload) {
  int32_t ptr = (int32_t) &RET_AREA;
  __wasm_import_outbound_redis_publish((int32_t) (*address).ptr, (int32_t) (*address).len, (int32_t) (*channel).ptr, (int32_t) (*channel).len, (int32_t) (*payload).ptr, (int32_t) (*payload).len, ptr);
  outbound_redis_expected_void_error_t variant;
  variant.tag = *((int32_t*) (ptr + 0));
  switch ((int32_t) variant.tag) {
    case 0: {
      break;
    }
    case 1: {
      variant.val.err = *((int32_t*) (ptr + 8));
      break;
    }
  }
  return variant.tag ? variant.val.err : -1;
}
__attribute__((import_module("outbound-redis"), import_name("get")))
void __wasm_import_outbound_redis_get(int32_t, int32_t, int32_t, int32_t, int32_t);
outbound_redis_error_t outbound_redis_get(outbound_redis_string_t *address, outbound_redis_string_t *key, outbound_redis_payload_t *ret0) {
  int32_t ptr = (int32_t) &RET_AREA;
  __wasm_import_outbound_redis_get((int32_t) (*address).ptr, (int32_t) (*address).len, (int32_t) (*key).ptr, (int32_t) (*key).len, ptr);
  outbound_redis_expected_payload_error_t variant;
  variant.tag = *((int32_t*) (ptr + 0));
  switch ((int32_t) variant.tag) {
    case 0: {
      variant.val.ok = (outbound_redis_payload_t) { (uint8_t*)(*((int32_t*) (ptr + 8))), (size_t)(*((int32_t*) (ptr + 16))) };
      break;
    }
    case 1: {
      variant.val.err = *((int32_t*) (ptr + 8));
      break;
    }
  }
  *ret0 = variant.val.ok;
  return variant.tag ? variant.val.err : -1;
}
__attribute__((import_module("outbound-redis"), import_name("set")))
void __wasm_import_outbound_redis_set(int32_t, int32_t, int32_t, int32_t, int32_t, int32_t, int32_t);
outbound_redis_error_t outbound_redis_set(outbound_redis_string_t *address, outbound_redis_string_t *key, outbound_redis_payload_t *value) {
  int32_t ptr = (int32_t) &RET_AREA;
  __wasm_import_outbound_redis_set((int32_t) (*address).ptr, (int32_t) (*address).len, (int32_t) (*key).ptr, (int32_t) (*key).len, (int32_t) (*value).ptr, (int32_t) (*value).len, ptr);
  outbound_redis_expected_void_error_t variant;
  variant.tag = *((int32_t*) (ptr + 0));
  switch ((int32_t) variant.tag) {
    case 0: {
      break;
    }
    case 1: {
      variant.val.err = *((int32_t*) (ptr + 8));
      break;
    }
  }
  return variant.tag ? variant.val.err : -1;
}
