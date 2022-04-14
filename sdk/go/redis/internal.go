package redis

// #include "spin-redis.h"
// #include<stdlib.h>
import "C"
import (
	"unsafe"
)

//export spin_redis_handle_redis_message
func handleRedisMessage(payload *C.spin_redis_payload_t) C.spin_redis_error_t {
	bytes := C.GoBytes(unsafe.Pointer(payload.ptr), C.int(payload.len))
	if err := handler(bytes); err != nil {
		return C.uint8_t(1)

	}
	return C.uint8_t(0)
}
