#ifndef __BINDINGS_SPIN_REDIS_H
#define __BINDINGS_SPIN_REDIS_H
#ifdef __cplusplus
extern "C"
{
  #endif
  
  #include <stdint.h>
  #include <stdbool.h>
  typedef uint8_t spin_redis_error_t;
  #define SPIN_REDIS_ERROR_SUCCESS 0
  #define SPIN_REDIS_ERROR_ERROR 1
  typedef struct {
    uint8_t *ptr;
    size_t len;
  } spin_redis_payload_t;
  void spin_redis_payload_free(spin_redis_payload_t *ptr);
  spin_redis_error_t spin_redis_handle_redis_message(spin_redis_payload_t *message);
  #ifdef __cplusplus
}
#endif
#endif
