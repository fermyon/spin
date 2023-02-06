#include <stdlib.h>
#include <spin-config.h>

__attribute__((weak, export_name("canonical_abi_realloc")))
void *canonical_abi_realloc(
void *ptr,
size_t orig_size,
size_t align,
size_t new_size
) {
  if (new_size == 0)
  return (void*) align;
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
  if (size == 0)
  return;
  free(ptr);
}
#include <string.h>

void spin_config_string_set(spin_config_string_t *ret, const char *s) {
  ret->ptr = (char*) s;
  ret->len = strlen(s);
}

void spin_config_string_dup(spin_config_string_t *ret, const char *s) {
  ret->len = strlen(s);
  ret->ptr = canonical_abi_realloc(NULL, 0, 1, ret->len);
  memcpy(ret->ptr, s, ret->len);
}

void spin_config_string_free(spin_config_string_t *ret) {
  canonical_abi_free(ret->ptr, ret->len, 1);
  ret->ptr = NULL;
  ret->len = 0;
}
void spin_config_error_free(spin_config_error_t *ptr) {
  switch ((int32_t) ptr->tag) {
    case 0: {
      spin_config_string_free(&ptr->val.provider);
      break;
    }
    case 1: {
      spin_config_string_free(&ptr->val.invalid_key);
      break;
    }
    case 2: {
      spin_config_string_free(&ptr->val.invalid_schema);
      break;
    }
    case 3: {
      spin_config_string_free(&ptr->val.other);
      break;
    }
  }
}
void spin_config_expected_string_error_free(spin_config_expected_string_error_t *ptr) {
  if (!ptr->is_err) {
    spin_config_string_free(&ptr->val.ok);
  } else {
    spin_config_error_free(&ptr->val.err);
  }
}

__attribute__((aligned(4)))
static uint8_t RET_AREA[16];
__attribute__((import_module("spin-config"), import_name("get-config")))
void __wasm_import_spin_config_get_config(int32_t, int32_t, int32_t);
void spin_config_get_config(spin_config_string_t *key, spin_config_expected_string_error_t *ret0) {
  int32_t ptr = (int32_t) &RET_AREA;
  __wasm_import_spin_config_get_config((int32_t) (*key).ptr, (int32_t) (*key).len, ptr);
  spin_config_expected_string_error_t expected;
  switch ((int32_t) (*((uint8_t*) (ptr + 0)))) {
    case 0: {
      expected.is_err = false;
      
      expected.val.ok = (spin_config_string_t) { (char*)(*((int32_t*) (ptr + 4))), (size_t)(*((int32_t*) (ptr + 8))) };
      break;
    }
    case 1: {
      expected.is_err = true;
      spin_config_error_t variant;
      variant.tag = (int32_t) (*((uint8_t*) (ptr + 4)));
      switch ((int32_t) variant.tag) {
        case 0: {
          variant.val.provider = (spin_config_string_t) { (char*)(*((int32_t*) (ptr + 8))), (size_t)(*((int32_t*) (ptr + 12))) };
          break;
        }
        case 1: {
          variant.val.invalid_key = (spin_config_string_t) { (char*)(*((int32_t*) (ptr + 8))), (size_t)(*((int32_t*) (ptr + 12))) };
          break;
        }
        case 2: {
          variant.val.invalid_schema = (spin_config_string_t) { (char*)(*((int32_t*) (ptr + 8))), (size_t)(*((int32_t*) (ptr + 12))) };
          break;
        }
        case 3: {
          variant.val.other = (spin_config_string_t) { (char*)(*((int32_t*) (ptr + 8))), (size_t)(*((int32_t*) (ptr + 12))) };
          break;
        }
      }
      
      expected.val.err = variant;
      break;
    }
  }*ret0 = expected;
}
