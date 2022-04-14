#include <stdlib.h>
#include <spin-redis.h>

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
void spin_redis_payload_free(spin_redis_payload_t *ptr) {
  canonical_abi_free(ptr->ptr, ptr->len * 1, 1);
}
typedef struct {
  // 0 if `val` is `ok`, 1 otherwise
  uint8_t tag;
  union {
    spin_redis_error_t err;
  } val;
} spin_redis_expected_void_error_t;
static int64_t RET_AREA[2];
__attribute__((export_name("handle-redis-message")))
int32_t __wasm_export_spin_redis_handle_redis_message(int32_t arg, int32_t arg0) {
  spin_redis_payload_t arg1 = (spin_redis_payload_t) { (uint8_t*)(arg), (size_t)(arg0) };
  spin_redis_error_t ret = spin_redis_handle_redis_message(&arg1);
  
  spin_redis_expected_void_error_t ret2;
  if (ret <= 2) {
    ret2.tag = 1;
    ret2.val.err = ret;
  } else {
    ret2.tag = 0;
    
  }
  int32_t variant6;
  int32_t variant7;
  switch ((int32_t) (ret2).tag) {
    case 0: {
      variant6 = 0;
      variant7 = 0;
      break;
    }
    case 1: {
      const spin_redis_error_t *payload3 = &(ret2).val.err;
      int32_t variant;
      switch ((int32_t) *payload3) {
        case 0: {
          variant = 0;
          break;
        }
        case 1: {
          variant = 1;
          break;
        }
      }
      variant6 = 1;
      variant7 = variant;
      break;
    }
  }
  int32_t ptr = (int32_t) &RET_AREA;
  *((int32_t*)(ptr + 8)) = variant7;
  *((int32_t*)(ptr + 0)) = variant6;
  return ptr;
}
