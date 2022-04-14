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
  outbound_redis_error_t outbound_redis_publish(outbound_redis_string_t *address, outbound_redis_string_t *channel, outbound_redis_payload_t *payload);
  outbound_redis_error_t outbound_redis_get(outbound_redis_string_t *address, outbound_redis_string_t *key, outbound_redis_payload_t *ret0);
  outbound_redis_error_t outbound_redis_set(outbound_redis_string_t *address, outbound_redis_string_t *key, outbound_redis_payload_t *value);
  #ifdef __cplusplus
}
#endif
#endif
