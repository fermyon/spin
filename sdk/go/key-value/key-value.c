#include <stdlib.h>
#include <key-value.h>

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

void key_value_string_set(key_value_string_t *ret, const char *s) {
  ret->ptr = (char*) s;
  ret->len = strlen(s);
}

void key_value_string_dup(key_value_string_t *ret, const char *s) {
  ret->len = strlen(s);
  ret->ptr = canonical_abi_realloc(NULL, 0, 1, ret->len);
  memcpy(ret->ptr, s, ret->len);
}

void key_value_string_free(key_value_string_t *ret) {
  canonical_abi_free(ret->ptr, ret->len, 1);
  ret->ptr = NULL;
  ret->len = 0;
}
void key_value_error_free(key_value_error_t *ptr) {
  switch ((int32_t) ptr->tag) {
    case 5: {
      key_value_string_free(&ptr->val.io);
      break;
    }
  }
}
void key_value_expected_store_error_free(key_value_expected_store_error_t *ptr) {
  if (!ptr->is_err) {
  } else {
    key_value_error_free(&ptr->val.err);
  }
}
void key_value_list_u8_free(key_value_list_u8_t *ptr) {
  canonical_abi_free(ptr->ptr, ptr->len * 1, 1);
}
void key_value_expected_list_u8_error_free(key_value_expected_list_u8_error_t *ptr) {
  if (!ptr->is_err) {
    key_value_list_u8_free(&ptr->val.ok);
  } else {
    key_value_error_free(&ptr->val.err);
  }
}
void key_value_expected_unit_error_free(key_value_expected_unit_error_t *ptr) {
  if (!ptr->is_err) {
  } else {
    key_value_error_free(&ptr->val.err);
  }
}
void key_value_expected_bool_error_free(key_value_expected_bool_error_t *ptr) {
  if (!ptr->is_err) {
  } else {
    key_value_error_free(&ptr->val.err);
  }
}
void key_value_list_string_free(key_value_list_string_t *ptr) {
  for (size_t i = 0; i < ptr->len; i++) {
    key_value_string_free(&ptr->ptr[i]);
  }
  canonical_abi_free(ptr->ptr, ptr->len * 8, 4);
}
void key_value_expected_list_string_error_free(key_value_expected_list_string_error_t *ptr) {
  if (!ptr->is_err) {
    key_value_list_string_free(&ptr->val.ok);
  } else {
    key_value_error_free(&ptr->val.err);
  }
}

__attribute__((aligned(4)))
static uint8_t RET_AREA[16];
__attribute__((import_module("key-value"), import_name("open")))
void __wasm_import_key_value_open(int32_t, int32_t, int32_t);
void key_value_open(key_value_string_t *name, key_value_expected_store_error_t *ret0) {
  int32_t ptr = (int32_t) &RET_AREA;
  __wasm_import_key_value_open((int32_t) (*name).ptr, (int32_t) (*name).len, ptr);
  key_value_expected_store_error_t expected;
  switch ((int32_t) (*((uint8_t*) (ptr + 0)))) {
    case 0: {
      expected.is_err = false;
      
      expected.val.ok = (uint32_t) (*((int32_t*) (ptr + 4)));
      break;
    }
    case 1: {
      expected.is_err = true;
      key_value_error_t variant;
      variant.tag = (int32_t) (*((uint8_t*) (ptr + 4)));
      switch ((int32_t) variant.tag) {
        case 0: {
          break;
        }
        case 1: {
          break;
        }
        case 2: {
          break;
        }
        case 3: {
          break;
        }
        case 4: {
          break;
        }
        case 5: {
          variant.val.io = (key_value_string_t) { (char*)(*((int32_t*) (ptr + 8))), (size_t)(*((int32_t*) (ptr + 12))) };
          break;
        }
      }
      
      expected.val.err = variant;
      break;
    }
  }*ret0 = expected;
}
__attribute__((import_module("key-value"), import_name("get")))
void __wasm_import_key_value_get(int32_t, int32_t, int32_t, int32_t);
void key_value_get(key_value_store_t store, key_value_string_t *key, key_value_expected_list_u8_error_t *ret0) {
  int32_t ptr = (int32_t) &RET_AREA;
  __wasm_import_key_value_get((int32_t) (store), (int32_t) (*key).ptr, (int32_t) (*key).len, ptr);
  key_value_expected_list_u8_error_t expected;
  switch ((int32_t) (*((uint8_t*) (ptr + 0)))) {
    case 0: {
      expected.is_err = false;
      
      expected.val.ok = (key_value_list_u8_t) { (uint8_t*)(*((int32_t*) (ptr + 4))), (size_t)(*((int32_t*) (ptr + 8))) };
      break;
    }
    case 1: {
      expected.is_err = true;
      key_value_error_t variant;
      variant.tag = (int32_t) (*((uint8_t*) (ptr + 4)));
      switch ((int32_t) variant.tag) {
        case 0: {
          break;
        }
        case 1: {
          break;
        }
        case 2: {
          break;
        }
        case 3: {
          break;
        }
        case 4: {
          break;
        }
        case 5: {
          variant.val.io = (key_value_string_t) { (char*)(*((int32_t*) (ptr + 8))), (size_t)(*((int32_t*) (ptr + 12))) };
          break;
        }
      }
      
      expected.val.err = variant;
      break;
    }
  }*ret0 = expected;
}
__attribute__((import_module("key-value"), import_name("set")))
void __wasm_import_key_value_set(int32_t, int32_t, int32_t, int32_t, int32_t, int32_t);
void key_value_set(key_value_store_t store, key_value_string_t *key, key_value_list_u8_t *value, key_value_expected_unit_error_t *ret0) {
  int32_t ptr = (int32_t) &RET_AREA;
  __wasm_import_key_value_set((int32_t) (store), (int32_t) (*key).ptr, (int32_t) (*key).len, (int32_t) (*value).ptr, (int32_t) (*value).len, ptr);
  key_value_expected_unit_error_t expected;
  switch ((int32_t) (*((uint8_t*) (ptr + 0)))) {
    case 0: {
      expected.is_err = false;
      
      
      break;
    }
    case 1: {
      expected.is_err = true;
      key_value_error_t variant;
      variant.tag = (int32_t) (*((uint8_t*) (ptr + 4)));
      switch ((int32_t) variant.tag) {
        case 0: {
          break;
        }
        case 1: {
          break;
        }
        case 2: {
          break;
        }
        case 3: {
          break;
        }
        case 4: {
          break;
        }
        case 5: {
          variant.val.io = (key_value_string_t) { (char*)(*((int32_t*) (ptr + 8))), (size_t)(*((int32_t*) (ptr + 12))) };
          break;
        }
      }
      
      expected.val.err = variant;
      break;
    }
  }*ret0 = expected;
}
__attribute__((import_module("key-value"), import_name("delete")))
void __wasm_import_key_value_delete(int32_t, int32_t, int32_t, int32_t);
void key_value_delete(key_value_store_t store, key_value_string_t *key, key_value_expected_unit_error_t *ret0) {
  int32_t ptr = (int32_t) &RET_AREA;
  __wasm_import_key_value_delete((int32_t) (store), (int32_t) (*key).ptr, (int32_t) (*key).len, ptr);
  key_value_expected_unit_error_t expected;
  switch ((int32_t) (*((uint8_t*) (ptr + 0)))) {
    case 0: {
      expected.is_err = false;
      
      
      break;
    }
    case 1: {
      expected.is_err = true;
      key_value_error_t variant;
      variant.tag = (int32_t) (*((uint8_t*) (ptr + 4)));
      switch ((int32_t) variant.tag) {
        case 0: {
          break;
        }
        case 1: {
          break;
        }
        case 2: {
          break;
        }
        case 3: {
          break;
        }
        case 4: {
          break;
        }
        case 5: {
          variant.val.io = (key_value_string_t) { (char*)(*((int32_t*) (ptr + 8))), (size_t)(*((int32_t*) (ptr + 12))) };
          break;
        }
      }
      
      expected.val.err = variant;
      break;
    }
  }*ret0 = expected;
}
__attribute__((import_module("key-value"), import_name("exists")))
void __wasm_import_key_value_exists(int32_t, int32_t, int32_t, int32_t);
void key_value_exists(key_value_store_t store, key_value_string_t *key, key_value_expected_bool_error_t *ret0) {
  int32_t ptr = (int32_t) &RET_AREA;
  __wasm_import_key_value_exists((int32_t) (store), (int32_t) (*key).ptr, (int32_t) (*key).len, ptr);
  key_value_expected_bool_error_t expected;
  switch ((int32_t) (*((uint8_t*) (ptr + 0)))) {
    case 0: {
      expected.is_err = false;
      
      expected.val.ok = (int32_t) (*((uint8_t*) (ptr + 4)));
      break;
    }
    case 1: {
      expected.is_err = true;
      key_value_error_t variant;
      variant.tag = (int32_t) (*((uint8_t*) (ptr + 4)));
      switch ((int32_t) variant.tag) {
        case 0: {
          break;
        }
        case 1: {
          break;
        }
        case 2: {
          break;
        }
        case 3: {
          break;
        }
        case 4: {
          break;
        }
        case 5: {
          variant.val.io = (key_value_string_t) { (char*)(*((int32_t*) (ptr + 8))), (size_t)(*((int32_t*) (ptr + 12))) };
          break;
        }
      }
      
      expected.val.err = variant;
      break;
    }
  }*ret0 = expected;
}
__attribute__((import_module("key-value"), import_name("get-keys")))
void __wasm_import_key_value_get_keys(int32_t, int32_t);
void key_value_get_keys(key_value_store_t store, key_value_expected_list_string_error_t *ret0) {
  int32_t ptr = (int32_t) &RET_AREA;
  __wasm_import_key_value_get_keys((int32_t) (store), ptr);
  key_value_expected_list_string_error_t expected;
  switch ((int32_t) (*((uint8_t*) (ptr + 0)))) {
    case 0: {
      expected.is_err = false;
      
      expected.val.ok = (key_value_list_string_t) { (key_value_string_t*)(*((int32_t*) (ptr + 4))), (size_t)(*((int32_t*) (ptr + 8))) };
      break;
    }
    case 1: {
      expected.is_err = true;
      key_value_error_t variant;
      variant.tag = (int32_t) (*((uint8_t*) (ptr + 4)));
      switch ((int32_t) variant.tag) {
        case 0: {
          break;
        }
        case 1: {
          break;
        }
        case 2: {
          break;
        }
        case 3: {
          break;
        }
        case 4: {
          break;
        }
        case 5: {
          variant.val.io = (key_value_string_t) { (char*)(*((int32_t*) (ptr + 8))), (size_t)(*((int32_t*) (ptr + 12))) };
          break;
        }
      }
      
      expected.val.err = variant;
      break;
    }
  }*ret0 = expected;
}
__attribute__((import_module("key-value"), import_name("close")))
void __wasm_import_key_value_close(int32_t);
void key_value_close(key_value_store_t store) {
  __wasm_import_key_value_close((int32_t) (store));
}
