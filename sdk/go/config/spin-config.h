#ifndef __BINDINGS_SPIN_CONFIG_H
#define __BINDINGS_SPIN_CONFIG_H
#ifdef __cplusplus
extern "C"
{
  #endif
  
  #include <stdint.h>
  #include <stdbool.h>
  
  typedef struct {
    char *ptr;
    size_t len;
  } spin_config_string_t;
  
  void spin_config_string_set(spin_config_string_t *ret, const char *s);
  void spin_config_string_dup(spin_config_string_t *ret, const char *s);
  void spin_config_string_free(spin_config_string_t *ret);
  typedef struct {
    uint8_t tag;
    union {
      spin_config_string_t provider;
      spin_config_string_t invalid_key;
      spin_config_string_t invalid_schema;
      spin_config_string_t other;
    } val;
  } spin_config_error_t;
  #define SPIN_CONFIG_ERROR_PROVIDER 0
  #define SPIN_CONFIG_ERROR_INVALID_KEY 1
  #define SPIN_CONFIG_ERROR_INVALID_SCHEMA 2
  #define SPIN_CONFIG_ERROR_OTHER 3
  void spin_config_error_free(spin_config_error_t *ptr);
  typedef struct {
    bool is_err;
    union {
      spin_config_string_t ok;
      spin_config_error_t err;
    } val;
  } spin_config_expected_string_error_t;
  void spin_config_expected_string_error_free(spin_config_expected_string_error_t *ptr);
  void spin_config_get_config(spin_config_string_t *key, spin_config_expected_string_error_t *ret0);
  #ifdef __cplusplus
}
#endif
#endif
