#ifndef __BINDINGS_OUTBOUND_REDIS_H
#define __BINDINGS_OUTBOUND_REDIS_H
#ifdef __cplusplus
extern "C"
{
  #endif
  
  #include <stdint.h>
  #include <stdbool.h>
  
  typedef struct {
    char *ptr;
    size_t len;
  } outbound_redis_string_t;
  
  void outbound_redis_string_set(outbound_redis_string_t *ret, const char *s);
  void outbound_redis_string_dup(outbound_redis_string_t *ret, const char *s);
  void outbound_redis_string_free(outbound_redis_string_t *ret);
  typedef uint8_t outbound_redis_error_t;
  #define OUTBOUND_REDIS_ERROR_SUCCESS 0
  #define OUTBOUND_REDIS_ERROR_ERROR 1
  typedef struct {
    uint8_t *ptr;
    size_t len;
  } outbound_redis_payload_t;
  void outbound_redis_payload_free(outbound_redis_payload_t *ptr);
  typedef struct {
    uint8_t tag;
    union {
      int64_t int64;
      outbound_redis_payload_t binary;
    } val;
  } outbound_redis_redis_parameter_t;
  #define OUTBOUND_REDIS_REDIS_PARAMETER_INT64 0
  #define OUTBOUND_REDIS_REDIS_PARAMETER_BINARY 1
  void outbound_redis_redis_parameter_free(outbound_redis_redis_parameter_t *ptr);
  typedef struct {
    uint8_t tag;
    union {
      outbound_redis_string_t status;
      int64_t int64;
      outbound_redis_payload_t binary;
    } val;
  } outbound_redis_redis_result_t;
  #define OUTBOUND_REDIS_REDIS_RESULT_NIL 0
  #define OUTBOUND_REDIS_REDIS_RESULT_STATUS 1
  #define OUTBOUND_REDIS_REDIS_RESULT_INT64 2
  #define OUTBOUND_REDIS_REDIS_RESULT_BINARY 3
  void outbound_redis_redis_result_free(outbound_redis_redis_result_t *ptr);
  typedef struct {
    outbound_redis_string_t *ptr;
    size_t len;
  } outbound_redis_list_string_t;
  void outbound_redis_list_string_free(outbound_redis_list_string_t *ptr);
  typedef struct {
    outbound_redis_redis_parameter_t *ptr;
    size_t len;
  } outbound_redis_list_redis_parameter_t;
  void outbound_redis_list_redis_parameter_free(outbound_redis_list_redis_parameter_t *ptr);
  typedef struct {
    outbound_redis_redis_result_t *ptr;
    size_t len;
  } outbound_redis_list_redis_result_t;
  void outbound_redis_list_redis_result_free(outbound_redis_list_redis_result_t *ptr);
  outbound_redis_error_t outbound_redis_publish(outbound_redis_string_t *address, outbound_redis_string_t *channel, outbound_redis_payload_t *payload);
  outbound_redis_error_t outbound_redis_get(outbound_redis_string_t *address, outbound_redis_string_t *key, outbound_redis_payload_t *ret0);
  outbound_redis_error_t outbound_redis_set(outbound_redis_string_t *address, outbound_redis_string_t *key, outbound_redis_payload_t *value);
  outbound_redis_error_t outbound_redis_incr(outbound_redis_string_t *address, outbound_redis_string_t *key, int64_t *ret0);
  outbound_redis_error_t outbound_redis_del(outbound_redis_string_t *address, outbound_redis_list_string_t *keys, int64_t *ret0);
  outbound_redis_error_t outbound_redis_sadd(outbound_redis_string_t *address, outbound_redis_string_t *key, outbound_redis_list_string_t *values, int64_t *ret0);
  outbound_redis_error_t outbound_redis_smembers(outbound_redis_string_t *address, outbound_redis_string_t *key, outbound_redis_list_string_t *ret0);
  outbound_redis_error_t outbound_redis_srem(outbound_redis_string_t *address, outbound_redis_string_t *key, outbound_redis_list_string_t *values, int64_t *ret0);
  outbound_redis_error_t outbound_redis_execute(outbound_redis_string_t *address, outbound_redis_string_t *command, outbound_redis_list_redis_parameter_t *arguments, outbound_redis_list_redis_result_t *ret0);
  #ifdef __cplusplus
}
#endif
#endif
