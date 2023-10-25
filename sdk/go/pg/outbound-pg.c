#include <stdlib.h>
#include <outbound-pg.h>

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

void outbound_pg_string_set(outbound_pg_string_t *ret, const char *s) {
  ret->ptr = (char*) s;
  ret->len = strlen(s);
}

void outbound_pg_string_dup(outbound_pg_string_t *ret, const char *s) {
  ret->len = strlen(s);
  ret->ptr = canonical_abi_realloc(NULL, 0, 1, ret->len);
  memcpy(ret->ptr, s, ret->len);
}

void outbound_pg_string_free(outbound_pg_string_t *ret) {
  canonical_abi_free(ret->ptr, ret->len, 1);
  ret->ptr = NULL;
  ret->len = 0;
}
void outbound_pg_pg_error_free(outbound_pg_pg_error_t *ptr) {
  switch ((int32_t) ptr->tag) {
    case 1: {
      outbound_pg_string_free(&ptr->val.connection_failed);
      break;
    }
    case 2: {
      outbound_pg_string_free(&ptr->val.bad_parameter);
      break;
    }
    case 3: {
      outbound_pg_string_free(&ptr->val.query_failed);
      break;
    }
    case 4: {
      outbound_pg_string_free(&ptr->val.value_conversion_failed);
      break;
    }
    case 5: {
      outbound_pg_string_free(&ptr->val.other_error);
      break;
    }
  }
}
void outbound_pg_column_free(outbound_pg_column_t *ptr) {
  outbound_pg_string_free(&ptr->name);
}
void outbound_pg_list_u8_free(outbound_pg_list_u8_t *ptr) {
  canonical_abi_free(ptr->ptr, ptr->len * 1, 1);
}
void outbound_pg_db_value_free(outbound_pg_db_value_t *ptr) {
  switch ((int32_t) ptr->tag) {
    case 11: {
      outbound_pg_string_free(&ptr->val.str);
      break;
    }
    case 12: {
      outbound_pg_list_u8_free(&ptr->val.binary);
      break;
    }
  }
}
void outbound_pg_parameter_value_free(outbound_pg_parameter_value_t *ptr) {
  switch ((int32_t) ptr->tag) {
    case 11: {
      outbound_pg_string_free(&ptr->val.str);
      break;
    }
    case 12: {
      outbound_pg_list_u8_free(&ptr->val.binary);
      break;
    }
  }
}
void outbound_pg_row_free(outbound_pg_row_t *ptr) {
  for (size_t i = 0; i < ptr->len; i++) {
    outbound_pg_db_value_free(&ptr->ptr[i]);
  }
  canonical_abi_free(ptr->ptr, ptr->len * 16, 8);
}
void outbound_pg_list_column_free(outbound_pg_list_column_t *ptr) {
  for (size_t i = 0; i < ptr->len; i++) {
    outbound_pg_column_free(&ptr->ptr[i]);
  }
  canonical_abi_free(ptr->ptr, ptr->len * 12, 4);
}
void outbound_pg_list_row_free(outbound_pg_list_row_t *ptr) {
  for (size_t i = 0; i < ptr->len; i++) {
    outbound_pg_row_free(&ptr->ptr[i]);
  }
  canonical_abi_free(ptr->ptr, ptr->len * 8, 4);
}
void outbound_pg_row_set_free(outbound_pg_row_set_t *ptr) {
  outbound_pg_list_column_free(&ptr->columns);
  outbound_pg_list_row_free(&ptr->rows);
}
void outbound_pg_list_parameter_value_free(outbound_pg_list_parameter_value_t *ptr) {
  for (size_t i = 0; i < ptr->len; i++) {
    outbound_pg_parameter_value_free(&ptr->ptr[i]);
  }
  canonical_abi_free(ptr->ptr, ptr->len * 16, 8);
}
void outbound_pg_expected_row_set_pg_error_free(outbound_pg_expected_row_set_pg_error_t *ptr) {
  if (!ptr->is_err) {
    outbound_pg_row_set_free(&ptr->val.ok);
  } else {
    outbound_pg_pg_error_free(&ptr->val.err);
  }
}
void outbound_pg_expected_u64_pg_error_free(outbound_pg_expected_u64_pg_error_t *ptr) {
  if (!ptr->is_err) {
  } else {
    outbound_pg_pg_error_free(&ptr->val.err);
  }
}

__attribute__((aligned(8)))
static uint8_t RET_AREA[20];
__attribute__((import_module("outbound-pg"), import_name("query")))
void __wasm_import_outbound_pg_query(int32_t, int32_t, int32_t, int32_t, int32_t, int32_t, int32_t);
void outbound_pg_query(outbound_pg_string_t *address, outbound_pg_string_t *statement, outbound_pg_list_parameter_value_t *params, outbound_pg_expected_row_set_pg_error_t *ret0) {
  int32_t ptr = (int32_t) &RET_AREA;
  __wasm_import_outbound_pg_query((int32_t) (*address).ptr, (int32_t) (*address).len, (int32_t) (*statement).ptr, (int32_t) (*statement).len, (int32_t) (*params).ptr, (int32_t) (*params).len, ptr);
  outbound_pg_expected_row_set_pg_error_t expected;
  switch ((int32_t) (*((uint8_t*) (ptr + 0)))) {
    case 0: {
      expected.is_err = false;
      
      expected.val.ok = (outbound_pg_row_set_t) {
        (outbound_pg_list_column_t) { (outbound_pg_column_t*)(*((int32_t*) (ptr + 4))), (size_t)(*((int32_t*) (ptr + 8))) },
        (outbound_pg_list_row_t) { (outbound_pg_row_t*)(*((int32_t*) (ptr + 12))), (size_t)(*((int32_t*) (ptr + 16))) },
      };
      break;
    }
    case 1: {
      expected.is_err = true;
      outbound_pg_pg_error_t variant13;
      variant13.tag = (int32_t) (*((uint8_t*) (ptr + 4)));
      switch ((int32_t) variant13.tag) {
        case 0: {
          break;
        }
        case 1: {
          variant13.val.connection_failed = (outbound_pg_string_t) { (char*)(*((int32_t*) (ptr + 8))), (size_t)(*((int32_t*) (ptr + 12))) };
          break;
        }
        case 2: {
          variant13.val.bad_parameter = (outbound_pg_string_t) { (char*)(*((int32_t*) (ptr + 8))), (size_t)(*((int32_t*) (ptr + 12))) };
          break;
        }
        case 3: {
          variant13.val.query_failed = (outbound_pg_string_t) { (char*)(*((int32_t*) (ptr + 8))), (size_t)(*((int32_t*) (ptr + 12))) };
          break;
        }
        case 4: {
          variant13.val.value_conversion_failed = (outbound_pg_string_t) { (char*)(*((int32_t*) (ptr + 8))), (size_t)(*((int32_t*) (ptr + 12))) };
          break;
        }
        case 5: {
          variant13.val.other_error = (outbound_pg_string_t) { (char*)(*((int32_t*) (ptr + 8))), (size_t)(*((int32_t*) (ptr + 12))) };
          break;
        }
      }
      
      expected.val.err = variant13;
      break;
    }
  }*ret0 = expected;
}
__attribute__((import_module("outbound-pg"), import_name("execute")))
void __wasm_import_outbound_pg_execute(int32_t, int32_t, int32_t, int32_t, int32_t, int32_t, int32_t);
void outbound_pg_execute(outbound_pg_string_t *address, outbound_pg_string_t *statement, outbound_pg_list_parameter_value_t *params, outbound_pg_expected_u64_pg_error_t *ret0) {
  int32_t ptr = (int32_t) &RET_AREA;
  __wasm_import_outbound_pg_execute((int32_t) (*address).ptr, (int32_t) (*address).len, (int32_t) (*statement).ptr, (int32_t) (*statement).len, (int32_t) (*params).ptr, (int32_t) (*params).len, ptr);
  outbound_pg_expected_u64_pg_error_t expected;
  switch ((int32_t) (*((uint8_t*) (ptr + 0)))) {
    case 0: {
      expected.is_err = false;
      
      expected.val.ok = (uint64_t) (*((int64_t*) (ptr + 8)));
      break;
    }
    case 1: {
      expected.is_err = true;
      outbound_pg_pg_error_t variant;
      variant.tag = (int32_t) (*((uint8_t*) (ptr + 8)));
      switch ((int32_t) variant.tag) {
        case 0: {
          break;
        }
        case 1: {
          variant.val.connection_failed = (outbound_pg_string_t) { (char*)(*((int32_t*) (ptr + 12))), (size_t)(*((int32_t*) (ptr + 16))) };
          break;
        }
        case 2: {
          variant.val.bad_parameter = (outbound_pg_string_t) { (char*)(*((int32_t*) (ptr + 12))), (size_t)(*((int32_t*) (ptr + 16))) };
          break;
        }
        case 3: {
          variant.val.query_failed = (outbound_pg_string_t) { (char*)(*((int32_t*) (ptr + 12))), (size_t)(*((int32_t*) (ptr + 16))) };
          break;
        }
        case 4: {
          variant.val.value_conversion_failed = (outbound_pg_string_t) { (char*)(*((int32_t*) (ptr + 12))), (size_t)(*((int32_t*) (ptr + 16))) };
          break;
        }
        case 5: {
          variant.val.other_error = (outbound_pg_string_t) { (char*)(*((int32_t*) (ptr + 12))), (size_t)(*((int32_t*) (ptr + 16))) };
          break;
        }
      }
      
      expected.val.err = variant;
      break;
    }
  }*ret0 = expected;
}
