#ifndef __BINDINGS_SPIN_REDIS_H
#define __BINDINGS_SPIN_REDIS_H
#ifdef __cplusplus
extern "C"
{
  #endif
  
  #include <stdint.h>
  #include <stdbool.h>
  
  typedef struct {
    char *ptr;
    size_t len;
  } spin_redis_string_t;
  
  void spin_redis_string_set(spin_redis_string_t *ret, const char *s);
  void spin_redis_string_dup(spin_redis_string_t *ret, const char *s);
  void spin_redis_string_free(spin_redis_string_t *ret);
  typedef uint8_t spin_redis_error_t;
  #define SPIN_REDIS_ERROR_SUCCESS 0
  #define SPIN_REDIS_ERROR_ERROR 1
  typedef struct {
    uint8_t *ptr;
    size_t len;
  } spin_redis_payload_t;
  void spin_redis_payload_free(spin_redis_payload_t *ptr);
  typedef struct {
    uint8_t tag;
    union {
      int64_t int64;
      spin_redis_payload_t binary;
    } val;
  } spin_redis_redis_parameter_t;
  #define SPIN_REDIS_REDIS_PARAMETER_INT64 0
  #define SPIN_REDIS_REDIS_PARAMETER_BINARY 1
  void spin_redis_redis_parameter_free(spin_redis_redis_parameter_t *ptr);
  typedef struct {
    uint8_t tag;
    union {
      spin_redis_string_t status;
      int64_t int64;
      spin_redis_payload_t binary;
    } val;
  } spin_redis_redis_result_t;
  #define SPIN_REDIS_REDIS_RESULT_NIL 0
  #define SPIN_REDIS_REDIS_RESULT_STATUS 1
  #define SPIN_REDIS_REDIS_RESULT_INT64 2
  #define SPIN_REDIS_REDIS_RESULT_BINARY 3
  void spin_redis_redis_result_free(spin_redis_redis_result_t *ptr);
  spin_redis_error_t spin_redis_handle_redis_message(spin_redis_payload_t *message);
  #ifdef __cplusplus
}
#endif
#endif
