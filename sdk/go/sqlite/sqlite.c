#include <stdlib.h>
#include <sqlite.h>

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

void sqlite_string_set(sqlite_string_t *ret, const char *s) {
  ret->ptr = (char*) s;
  ret->len = strlen(s);
}

void sqlite_string_dup(sqlite_string_t *ret, const char *s) {
  ret->len = strlen(s);
  ret->ptr = canonical_abi_realloc(NULL, 0, 1, ret->len);
  memcpy(ret->ptr, s, ret->len);
}

void sqlite_string_free(sqlite_string_t *ret) {
  canonical_abi_free(ret->ptr, ret->len, 1);
  ret->ptr = NULL;
  ret->len = 0;
}
void sqlite_error_free(sqlite_error_t *ptr) {
  switch ((int32_t) ptr->tag) {
    case 4: {
      sqlite_string_free(&ptr->val.io);
      break;
    }
  }
}
void sqlite_list_string_free(sqlite_list_string_t *ptr) {
  for (size_t i = 0; i < ptr->len; i++) {
    sqlite_string_free(&ptr->ptr[i]);
  }
  canonical_abi_free(ptr->ptr, ptr->len * 8, 4);
}
void sqlite_list_u8_free(sqlite_list_u8_t *ptr) {
  canonical_abi_free(ptr->ptr, ptr->len * 1, 1);
}
void sqlite_value_free(sqlite_value_t *ptr) {
  switch ((int32_t) ptr->tag) {
    case 2: {
      sqlite_string_free(&ptr->val.text);
      break;
    }
    case 3: {
      sqlite_list_u8_free(&ptr->val.blob);
      break;
    }
  }
}
void sqlite_list_value_free(sqlite_list_value_t *ptr) {
  for (size_t i = 0; i < ptr->len; i++) {
    sqlite_value_free(&ptr->ptr[i]);
  }
  canonical_abi_free(ptr->ptr, ptr->len * 16, 8);
}
void sqlite_row_result_free(sqlite_row_result_t *ptr) {
  sqlite_list_value_free(&ptr->values);
}
void sqlite_list_row_result_free(sqlite_list_row_result_t *ptr) {
  for (size_t i = 0; i < ptr->len; i++) {
    sqlite_row_result_free(&ptr->ptr[i]);
  }
  canonical_abi_free(ptr->ptr, ptr->len * 8, 4);
}
void sqlite_query_result_free(sqlite_query_result_t *ptr) {
  sqlite_list_string_free(&ptr->columns);
  sqlite_list_row_result_free(&ptr->rows);
}
void sqlite_expected_connection_error_free(sqlite_expected_connection_error_t *ptr) {
  if (!ptr->is_err) {
  } else {
    sqlite_error_free(&ptr->val.err);
  }
}
void sqlite_expected_query_result_error_free(sqlite_expected_query_result_error_t *ptr) {
  if (!ptr->is_err) {
    sqlite_query_result_free(&ptr->val.ok);
  } else {
    sqlite_error_free(&ptr->val.err);
  }
}

__attribute__((aligned(4)))
static uint8_t RET_AREA[20];
__attribute__((import_module("sqlite"), import_name("open")))
void __wasm_import_sqlite_open(int32_t, int32_t, int32_t);
void sqlite_open(sqlite_string_t *name, sqlite_expected_connection_error_t *ret0) {
  int32_t ptr = (int32_t) &RET_AREA;
  __wasm_import_sqlite_open((int32_t) (*name).ptr, (int32_t) (*name).len, ptr);
  sqlite_expected_connection_error_t expected;
  switch ((int32_t) (*((uint8_t*) (ptr + 0)))) {
    case 0: {
      expected.is_err = false;
      
      expected.val.ok = (uint32_t) (*((int32_t*) (ptr + 4)));
      break;
    }
    case 1: {
      expected.is_err = true;
      sqlite_error_t variant;
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
          variant.val.io = (sqlite_string_t) { (char*)(*((int32_t*) (ptr + 8))), (size_t)(*((int32_t*) (ptr + 12))) };
          break;
        }
      }
      
      expected.val.err = variant;
      break;
    }
  }*ret0 = expected;
}
__attribute__((import_module("sqlite"), import_name("execute")))
void __wasm_import_sqlite_execute(int32_t, int32_t, int32_t, int32_t, int32_t, int32_t);
void sqlite_execute(sqlite_connection_t conn, sqlite_string_t *statement, sqlite_list_value_t *parameters, sqlite_expected_query_result_error_t *ret0) {
  int32_t ptr = (int32_t) &RET_AREA;
  __wasm_import_sqlite_execute((int32_t) (conn), (int32_t) (*statement).ptr, (int32_t) (*statement).len, (int32_t) (*parameters).ptr, (int32_t) (*parameters).len, ptr);
  sqlite_expected_query_result_error_t expected;
  switch ((int32_t) (*((uint8_t*) (ptr + 0)))) {
    case 0: {
      expected.is_err = false;
      
      expected.val.ok = (sqlite_query_result_t) {
        (sqlite_list_string_t) { (sqlite_string_t*)(*((int32_t*) (ptr + 4))), (size_t)(*((int32_t*) (ptr + 8))) },
        (sqlite_list_row_result_t) { (sqlite_row_result_t*)(*((int32_t*) (ptr + 12))), (size_t)(*((int32_t*) (ptr + 16))) },
      };
      break;
    }
    case 1: {
      expected.is_err = true;
      sqlite_error_t variant4;
      variant4.tag = (int32_t) (*((uint8_t*) (ptr + 4)));
      switch ((int32_t) variant4.tag) {
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
          variant4.val.io = (sqlite_string_t) { (char*)(*((int32_t*) (ptr + 8))), (size_t)(*((int32_t*) (ptr + 12))) };
          break;
        }
      }
      
      expected.val.err = variant4;
      break;
    }
  }*ret0 = expected;
}
__attribute__((import_module("sqlite"), import_name("close")))
void __wasm_import_sqlite_close(int32_t);
void sqlite_close(sqlite_connection_t conn) {
  __wasm_import_sqlite_close((int32_t) (conn));
}
