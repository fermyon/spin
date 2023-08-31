#ifndef __BINDINGS_KEY_VALUE_H
#define __BINDINGS_KEY_VALUE_H
#ifdef __cplusplus
extern "C"
{
  #endif
  
  #include <stdint.h>
  #include <stdbool.h>
  
  typedef struct {
    char *ptr;
    size_t len;
  } key_value_string_t;
  
  void key_value_string_set(key_value_string_t *ret, const char *s);
  void key_value_string_dup(key_value_string_t *ret, const char *s);
  void key_value_string_free(key_value_string_t *ret);
  typedef uint32_t key_value_store_t;
  typedef struct {
    uint8_t tag;
    union {
      key_value_string_t io;
    } val;
  } key_value_error_t;
  #define KEY_VALUE_ERROR_STORE_TABLE_FULL 0
  #define KEY_VALUE_ERROR_NO_SUCH_STORE 1
  #define KEY_VALUE_ERROR_ACCESS_DENIED 2
  #define KEY_VALUE_ERROR_INVALID_STORE 3
  #define KEY_VALUE_ERROR_NO_SUCH_KEY 4
  #define KEY_VALUE_ERROR_IO 5
  void key_value_error_free(key_value_error_t *ptr);
  typedef struct {
    bool is_err;
    union {
      key_value_store_t ok;
      key_value_error_t err;
    } val;
  } key_value_expected_store_error_t;
  void key_value_expected_store_error_free(key_value_expected_store_error_t *ptr);
  typedef struct {
    uint8_t *ptr;
    size_t len;
  } key_value_list_u8_t;
  void key_value_list_u8_free(key_value_list_u8_t *ptr);
  typedef struct {
    bool is_err;
    union {
      key_value_list_u8_t ok;
      key_value_error_t err;
    } val;
  } key_value_expected_list_u8_error_t;
  void key_value_expected_list_u8_error_free(key_value_expected_list_u8_error_t *ptr);
  typedef struct {
    bool is_err;
    union {
      key_value_error_t err;
    } val;
  } key_value_expected_unit_error_t;
  void key_value_expected_unit_error_free(key_value_expected_unit_error_t *ptr);
  typedef struct {
    bool is_err;
    union {
      bool ok;
      key_value_error_t err;
    } val;
  } key_value_expected_bool_error_t;
  void key_value_expected_bool_error_free(key_value_expected_bool_error_t *ptr);
  typedef struct {
    key_value_string_t *ptr;
    size_t len;
  } key_value_list_string_t;
  void key_value_list_string_free(key_value_list_string_t *ptr);
  typedef struct {
    bool is_err;
    union {
      key_value_list_string_t ok;
      key_value_error_t err;
    } val;
  } key_value_expected_list_string_error_t;
  void key_value_expected_list_string_error_free(key_value_expected_list_string_error_t *ptr);
  void key_value_open(key_value_string_t *name, key_value_expected_store_error_t *ret0);
  void key_value_get(key_value_store_t store, key_value_string_t *key, key_value_expected_list_u8_error_t *ret0);
  void key_value_set(key_value_store_t store, key_value_string_t *key, key_value_list_u8_t *value, key_value_expected_unit_error_t *ret0);
  void key_value_delete(key_value_store_t store, key_value_string_t *key, key_value_expected_unit_error_t *ret0);
  void key_value_exists(key_value_store_t store, key_value_string_t *key, key_value_expected_bool_error_t *ret0);
  void key_value_get_keys(key_value_store_t store, key_value_expected_list_string_error_t *ret0);
  void key_value_close(key_value_store_t store);
  #ifdef __cplusplus
}
#endif
#endif
